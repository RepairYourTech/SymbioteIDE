#!/usr/bin/env python3
"""Idempotently create/update the Symbiote v2 roadmap issues.

The plan is intentionally encoded as data plus category contracts. Every issue is
identified by a stable HTML marker. Re-running updates the managed issue bodies
and labels but never closes or deletes unrelated work.
"""
from __future__ import annotations

import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from typing import Iterable

REPO = os.environ.get("GITHUB_REPOSITORY", "RepairYourTech/SymbioteIDE")
TOKEN = os.environ.get("GITHUB_TOKEN", "")
API = os.environ.get("GITHUB_API_URL", "https://api.github.com")
DRY_RUN = os.environ.get("DRY_RUN", "0") == "1"
MARKER_RE = re.compile(r"<!--\s*symbiote-plan-key:\s*([^\s]+)\s*-->")

if "/" not in REPO:
    raise SystemExit(f"Invalid GITHUB_REPOSITORY: {REPO!r}")
OWNER, NAME = REPO.split("/", 1)

LABELS = {
    "symbiote-roadmap": ("5319e7", "Managed by the Symbiote v2 roadmap bootstrap"),
    "type:master": ("0e8a16", "Top-level program tracker"),
    "type:epic": ("1d76db", "Epic progress tracker"),
    "type:feature": ("0052cc", "Atomic implementation capability"),
    "type:research": ("d4c5f9", "Research, evidence, or architecture decision"),
    "priority:P0": ("b60205", "Foundation or launch-critical"),
    "priority:P1": ("fbca04", "Required product capability"),
    "priority:P2": ("cfd3d7", "Expansion after core conformance"),
    "wave:0": ("6f42c1", "Product truth and architecture"),
    "wave:1": ("8250df", "Core Host, project, protocol, and adapter foundations"),
    "wave:2": ("0366d6", "Orchestration, canonical work, and P0 harnesses"),
    "wave:3": ("1d76db", "System Graph, Context Broker, and Deep Guidance"),
    "wave:4": ("0e8a16", "Desktop workbench and continuous engineering"),
    "wave:5": ("5319e7", "Fabric, remote control, and mobile"),
    "wave:6": ("a2eeef", "Delivery, closure, GitHub automation, and advanced intelligence"),
    "wave:7": ("bfdadc", "Cross-platform release, hardening, and ecosystem"),
    "platform:linux": ("f9d0c4", "Linux reference implementation"),
    "platform:windows": ("0078d4", "Windows support"),
    "platform:macos": ("7057ff", "macOS support"),
    "platform:ios": ("c5def5", "iOS client"),
    "platform:android": ("3ddc84", "Android client"),
}

@dataclass
class Item:
    key: str
    title: str
    epic: str | None
    wave: int
    priority: str
    kind: str
    outcome: str
    scope: list[str]
    deps: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    platforms: list[str] = field(default_factory=list)

EPICS: dict[str, dict] = {}
ITEMS: list[Item] = []


def epic(key: str, title: str, purpose: str, wave: int) -> None:
    EPICS[key] = {"key": key, "title": title, "purpose": purpose, "wave": wave}


def add(key: str, title: str, epic_key: str, wave: int, priority: str, kind: str,
        outcome: str, scope: Iterable[str], deps: Iterable[str] = (),
        notes: Iterable[str] = (), platforms: Iterable[str] = ()) -> None:
    ITEMS.append(Item(key, title, epic_key, wave, priority, kind, outcome,
                      list(scope), list(deps), list(notes), list(platforms)))

# ---------------------------------------------------------------------------
# Epic definitions
# ---------------------------------------------------------------------------
epic("E00", "Product truth, competitor intelligence, and governing architecture",
      "Keep Symbiote grounded in a current market baseline, explicit invariants, open-source trust, and evidence-backed architectural decisions.", 0)
epic("E01", "Projects, workspaces, persistence, and configuration",
      "Make a Project a true dedicated operating context rather than a session folder, with portable settings and resource-aware intelligence lifecycle.", 1)
epic("E02", "Linux Host, Fabric, client protocol, and distributed execution",
      "Establish Linux-first symbioted Hosts, multi-client control, trusted Host swarms, placement, and transport-neutral distributed execution.", 1)
epic("E03", "Harness runtime platform and compatibility packs",
      "Support coding harnesses through truthful capability contracts and native configuration projections instead of terminal scraping or lowest-common-denominator assumptions.", 1)
epic("E04", "AI Team, Lead orchestrator, routing, and canonical work",
      "Let the client describe outcomes while project Roles, the selected Lead, and Symbiote's deterministic control plane plan and execute the work.", 2)
epic("E05", "Sessions, artifacts, annotations, and activity provenance",
      "Preserve engineering work as structured, versioned, reviewable institutional knowledge rather than disposable chat transcripts.", 2)
epic("E06", "System Graph and hybrid code-intelligence foundation",
      "Build a persistent evidence-backed digital twin spanning product intent, code, data, runtime, infrastructure, history, tests, and agent work.", 3)
epic("E07", "Context Broker, blast radius, and graph intelligence applications",
      "Use the graph for precise context, change completeness, dead-code discovery, affected tests, policy enforcement, and safe parallelism.", 3)
epic("E08", "Deep Guidance, competitor gaps, and ambiguity remediation",
      "Turn raw ideas into exhaustive implementation-safe specifications through native CFSA-derived methodology, competitor analysis, decision locking, and adversarial remediation.", 3)
epic("E09", "Delivery Architecture, Capability Closure, and production readiness",
      "Research how software should be hosted, distributed, operated, and verified; prevent test-green but underdeveloped capabilities from being called complete.", 3)
epic("E10", "Linux desktop shell and integrated development workbench",
      "Deliver the client-facing Nerve Center plus professional editor, terminals, Preview design surface, artifacts, graph, and diagnostics without permanent UI crowding.", 4)
epic("E11", "Git, worktrees, GitHub, CI, review, and merge automation",
      "Make issue-first, task-owned worktrees, PRs, independent review, checks, and graph-aware merging the default GitHub delivery path.", 2)
epic("E12", "MCPs, skills, secrets, native runtime environments, and extensions",
      "Provide one safe desired-state experience while preserving each harness's native files, scopes, precedence, trust model, and capabilities.", 2)
epic("E13", "iOS, Android, Symbiote Link, and remote client experience",
      "Allow full client-level control of one or many Hosts over LAN or remote internet from first-class mobile applications.", 5)
epic("E14", "Security, diagnostics, recovery, packaging, and cross-platform conformance",
      "Harden the firm for continuous unattended work and ship Linux first without compromising required Windows and macOS support.", 4)

# ---------------------------------------------------------------------------
# E00 — Product truth
# ---------------------------------------------------------------------------
add("A01", "Lock the Symbiote product constitution and non-negotiable invariants", "E00", 0, "P0", "architecture",
    "Create the authoritative product constitution that every specification, issue, adapter, and implementation must obey.", [
        "Define the client/AI-development-firm model and explicitly reject a session-centric multi-terminal product.",
        "Define the separation between the user-selected conversational Lead and Symbiote-owned canonical state, scheduling, policy, context, and evidence.",
        "Lock mandatory Role-to-harness/model assignment, Linux-first Host architecture, cross-platform clients, System Graph, Context Broker, Deep Guidance, and Capability Closure.",
        "Record non-goals: no proprietary required model, no forced inference reseller, no editor-fork dependency, and no transcript as source of truth.",
        "Create a change-control rule requiring explicit impact propagation when a constitutional decision changes."
    ])
add("A02", "Build the continuously refreshed ADE and coding-agent competitor registry", "E00", 0, "P0", "research",
    "Create a dated, source-backed registry of direct and adjacent competitors that can be refreshed throughout product development.", [
        "Track Orca, Traycer, BridgeMind One, T3 Code, ADE, Superset, Vibe Kanban, OpenCode Desktop, and newly discovered ADEs.",
        "Capture license, platforms, runtime architecture, supported harnesses, worktrees, artifacts, browser/preview, editor, terminals, mobile/remote, GitHub, voice, and monetization.",
        "Separate verified current capability, announced capability, experimental capability, and inference.",
        "Store source URL, retrieval date, product version/commit, and evidence excerpt for every nontrivial claim.",
        "Provide a scheduled/manual refresh workflow that preserves historical snapshots and highlights market movement."
    ])
add("A03", "Produce parity, gap, and strategic differentiation matrices", "E00", 0, "P0", "research",
    "Convert competitor evidence into launch parity requirements, borrowable interaction patterns, missing-market opportunities, and defensible Symbiote advantages.", [
        "Distinguish table stakes from differentiators instead of treating every competitor feature as equal.",
        "Analyze complete workflows, not isolated feature checkboxes: idea-to-spec, task-to-worktree, artifact review, mobile intervention, CI repair, and delivery.",
        "Create explicit borrow/adapt/reject decisions for BridgeMind panes and teammate identity, Traycer artifacts, Orca workbench breadth, T3 provider/remote architecture, and other strong patterns.",
        "Map every accepted gap to a requirement, epic, or documented deferral with rationale.",
        "Expose stale or unsupported product claims when competitor evidence changes."
    ], deps=["A02"])
add("A04", "Establish ADRs, spike governance, and evidence-backed architecture decisions", "E00", 0, "P0", "architecture",
    "Ensure consequential technical choices are tested and recorded rather than buried in chat or chosen by familiarity.", [
        "Define ADR schema, decision authority, alternatives, constraints, evidence, reversibility, consequences, and supersession.",
        "Create spike contracts with hypothesis, representative workload, measurement plan, stop condition, result artifact, and cleanup.",
        "Require research refresh for temporally unstable vendor, SDK, protocol, pricing, and platform decisions.",
        "Link ADRs to requirements, graph nodes, issues, implementation, and later outcome evidence.",
        "Prevent an implementation worker from silently converting an unresolved product or architecture question into code."
    ], deps=["A01"])
add("A05", "Define open-source licensing, plugin trust, and data-ownership policy", "E00", 0, "P0", "security",
    "Lock the legal and trust model required for an open-source tool that controls repositories, credentials, terminals, and third-party agents.", [
        "Select and document core, official-pack, extension, catalog-metadata, and hosted-service licensing boundaries.",
        "Define local-first data ownership, optional hosted services, telemetry opt-in, and no hidden code or prompt exfiltration.",
        "Define extension publisher identity, signing, permission review, revocation, and vulnerable-package response.",
        "Document contribution, trademark, fork, and commercial-hosting expectations.",
        "Create a dependency-license and generated-artifact provenance policy enforceable in CI."
    ], deps=["A01"])
add("A06", "Define success metrics, launch gates, and explicit non-goals by release", "E00", 0, "P1", "product",
    "Give the program measurable outcomes and stop uncontrolled scope from obscuring whether the core firm actually works.", [
        "Define quality, task success, context efficiency, graph precision/recall, recovery, latency, mobile control, and delivery metrics.",
        "Create release gates for technical preview, alpha, beta, and stable without weakening the production-quality bar inside shipped scope.",
        "Document what each release intentionally does not support and why.",
        "Require a golden end-to-end client request to production evidence path for every release candidate.",
        "Track regressions against competitor parity and constitutional invariants."
    ], deps=["A01", "A03"])

# ---------------------------------------------------------------------------
# E01 — Projects and configuration
# ---------------------------------------------------------------------------
add("B01", "Define canonical application, user, Project, workspace, repository, and machine configuration scopes", "E01", 1, "P0", "architecture",
    "Create an unambiguous configuration model before any client or harness adapter persists state.", [
        "Separate application defaults, user preferences, machine/Host state, Project policy, workspace roots, repository metadata, Role settings, and secrets.",
        "Define precedence, inheritance, overrides, provenance, portability, and conflict behavior for every scope.",
        "Keep canonical technical names in schemas and APIs regardless of branded terminology mode.",
        "Specify which values may be version-controlled, local-only, synchronized across clients, or prohibited from disk.",
        "Version the schema and require forward/backward migration tests."
    ], deps=["A01"])
add("B02", "Implement the Project registry, dropdown switching, and dedicated Project operating context", "E01", 1, "P0", "feature",
    "Make switching Projects replace the entire working environment rather than merely filtering session history.", [
        "Persist independent sessions, Team, Lead, task graph, artifacts, graph/index settings, terminals, layouts, Git state, resource policy, and recent UI state per Project.",
        "Support fast switching with explicit ACTIVE, WARM, SUSPENDED, and OFFLINE transitions.",
        "Prevent events, files, terminals, or agent state from one Project leaking into another Project UI.",
        "Restore the last active Project and its layout after restart without auto-starting disallowed resources.",
        "Expose clear unavailable/degraded states when a Project dependency or Host is offline."
    ], deps=["B01", "H01", "H02"])
add("B03", "Support multi-root, monorepo, and multi-repository Project workspaces", "E01", 1, "P0", "feature",
    "Model a logical software product independently from a single Git repository or directory.", [
        "Allow one or many roots with per-root repository, language, command, trust, and indexing metadata.",
        "Resolve symbol, contract, task, and dependency relationships across roots and repositories.",
        "Define canonical path identities that remain stable across Hosts and worktrees.",
        "Support repository-specific Git remotes and permissions under one Project objective/task graph.",
        "Detect overlapping, nested, missing, moved, and duplicate roots deterministically."
    ], deps=["B01", "M01"])
add("B04", "Create the versioned .symbiote Project manifest and portable desired-state model", "E01", 1, "P0", "feature",
    "Make Project intent reproducible while keeping secrets and machine-local state out of version control.", [
        "Represent Project identity, roots, commands, Team Roles, harness requirements, environment requirements, tool/skill intent, policies, graph profile, and delivery settings.",
        "Split portable declarations from generated state, caches, credentials, sessions, and Host-specific bindings.",
        "Provide schema validation, human-readable diagnostics, migrations, and deterministic serialization.",
        "Support partial manifests and explicit local overrides without silent precedence surprises.",
        "Generate a change impact preview before applying manifest migrations or destructive reconciliation."
    ], deps=["B01"])
add("B05", "Implement Project intelligence lifecycle and resource policies", "E01", 2, "P0", "feature",
    "Let users keep many indexed Projects without running every LSP, watcher, parser, embedding worker, and container continuously.", [
        "Implement ACTIVE, WARM, SUSPENDED, and OFFLINE service-state contracts with explicit persisted versus transient components.",
        "Support always-active, active-while-open, timed-warm, Host-resource-budget, and manual policies.",
        "Drain or checkpoint in-flight indexing safely before suspension and resume incrementally without a full rebuild when valid.",
        "Display memory, CPU, GPU, disk, process, and estimated wake-cost attribution by Project service.",
        "Never suspend execution required by an active task without policy-authorized migration or a visible blocked state."
    ], deps=["B02", "H06", "G12"])
add("B06", "Implement shared, isolated, and external SurrealDB Project profiles", "E01", 2, "P0", "feature",
    "Support efficient logical isolation by default and stronger process/version isolation when a Project needs it.", [
        "Provide shared-local namespace/database, isolated-local container/stack, and external/cloud connection profiles.",
        "Generate collision-resistant namespace/database identities and retain a human-readable mapping.",
        "Manage lifecycle, version compatibility, migrations, credentials, backup, health, and recovery per profile.",
        "Allow a user to move a Project between profiles through an export/import verification workflow.",
        "Make resource consequences and data location explicit before profile changes."
    ], deps=["B04", "G02", "N08"])
add("B07", "Implement Project import, restore, archive, clone, and terminology preferences", "E01", 3, "P1", "feature",
    "Provide safe lifecycle operations for existing repositories and portable Symbiote Projects.", [
        "Import an existing directory/repository without overwriting existing harness configuration or source files.",
        "Restore a manifest on a new Host with a missing-dependency plan before mutation.",
        "Archive versus delete with explicit effects on worktrees, graph data, sessions, artifacts, secrets, and remote references.",
        "Clone Project configuration without cloning secret material or stable external identities by accident.",
        "Offer Standard, Balanced, and Immersive terminology as a synchronized presentation preference while APIs remain canonical."
    ], deps=["B04", "N10", "H03"])

# ---------------------------------------------------------------------------
# E02 — Host and Fabric
# ---------------------------------------------------------------------------
add("H01", "Implement the Linux-first symbioted Host daemon and service lifecycle", "E02", 1, "P0", "host",
    "Create the headless reference runtime that owns Project services, processes, task execution, repositories, terminals, and agent harnesses independently of any GUI.", [
        "Run interactively for development and as a hardened user/system service with explicit data/config/cache/log directories.",
        "Expose health, version, capability, shutdown, restart, upgrade, and graceful-drain operations.",
        "Supervise child processes without orphaning harnesses, terminals, watchers, or dev servers.",
        "Recover persisted canonical state after daemon restart while marking unverifiable external processes correctly.",
        "Keep desktop-window lifecycle independent from Host execution lifecycle."
    ], deps=["A01"], platforms=["linux"])
add("H02", "Define and implement the versioned typed RPC and event protocol", "E02", 1, "P0", "protocol",
    "Make every desktop, web, mobile, and CLI controller a client of the same stable Host control plane.", [
        "Define request/response, streaming events, command IDs, idempotency keys, correlation, cancellation, pagination, errors, and capability negotiation.",
        "Separate durable domain events from ephemeral UI telemetry and raw harness streams.",
        "Provide protocol version negotiation and compatible deprecation rules.",
        "Enforce authorization per operation and resource rather than per connected socket alone.",
        "Generate or validate typed clients and contract tests from a canonical schema."
    ], deps=["H01", "B01"])
add("H03", "Build the shared client SDK with reconnect, snapshots, and event replay", "E02", 1, "P0", "protocol",
    "Give all clients one robust connection/state implementation instead of independently recreating synchronization logic.", [
        "Bootstrap from a consistent snapshot and resume from a durable event cursor after transient disconnects.",
        "Handle duplicate, delayed, out-of-order, unauthorized, and schema-version-mismatched events.",
        "Expose optimistic-command semantics only where conflict and rollback behavior are defined.",
        "Provide connection state, retry/backoff, offline cache boundaries, and resynchronization telemetry.",
        "Pass deterministic protocol chaos tests against the Linux Host."
    ], deps=["H02"])
add("H04", "Implement Host identity, cryptographic pairing, device credentials, and scoped sessions", "E02", 1, "P0", "security",
    "Establish durable trust without treating presence on a LAN as authorization.", [
        "Generate Host and client device identities with rotatable keys and human-verifiable pairing.",
        "Support QR and short-code pairing without embedding long-lived credentials in URLs.",
        "Mint short-lived API/session and WebSocket tickets from scoped device credentials.",
        "Support view, operate, source-write, terminal, merge, deploy, and Host-admin scopes with Project restrictions.",
        "Provide device listing, last use, revocation, expiration, and lost-device recovery."
    ], deps=["H02", "Q01"])
add("H05", "Implement zero-configuration LAN discovery with authenticated connection upgrade", "E02", 2, "P1", "network",
    "Let users discover and pair nearby Hosts without typing addresses while preserving an explicit trust boundary.", [
        "Advertise only non-sensitive discovery metadata through mDNS or a justified equivalent.",
        "Handle multiple interfaces, IPv4/IPv6, VPNs, duplicate names, sleep/wake, and changing addresses.",
        "Require H04 pairing before privileged RPC and prevent discovery spoofing from silently replacing known identity.",
        "Allow discovery to be disabled per Host and network profile.",
        "Provide manual address entry as a deterministic fallback."
    ], deps=["H03", "H04"])
add("H06", "Implement Host capability, harness, model, platform, and resource advertisement (Pulse)", "E02", 2, "P0", "host",
    "Create a truthful schedulable inventory for every Host in a Fabric.", [
        "Advertise OS/architecture, CPU, memory, GPU/VRAM or unified memory, disk, Docker/container support, build toolchains, harness instances, local models, inference servers, and Project services.",
        "Differentiate installed, authenticated, healthy, degraded, busy, rate-limited, unavailable, and unknown capabilities.",
        "Stream normalized load/health with bounded frequency and no credential leakage.",
        "Record capability provenance and freshness so the scheduler cannot act on stale assumptions.",
        "Support static user overrides only when marked and validated against runtime observations."
    ], deps=["H01", "C03"])
add("H07", "Implement Fabric membership, coordinator election, and multi-controller shared state", "E02", 3, "P0", "distributed",
    "Allow any trusted number of Hosts and clients to participate without one irreplaceable desktop master.", [
        "Define Fabric identity, membership, trust admission, removal, partition, and rejoin behavior.",
        "Separate authoritative Project/service placement from the client currently displaying it.",
        "Support preferred coordinator plus deterministic failover without split-brain task dispatch.",
        "Synchronize Lead sessions, tasks, artifacts, approvals, and activity across simultaneous desktop and mobile controllers.",
        "Define conflict arbitration for concurrent commands and surface rejected/stale actions clearly."
    ], deps=["H03", "H04", "D07", "F07"])
add("H08", "Implement Task → Role → Harness → Host execution placement", "E02", 3, "P0", "scheduler",
    "Select an eligible physical execution Host after logical Role and harness/model resolution.", [
        "Model hard requirements such as OS, Xcode, Windows toolchain, RAM, GPU, model availability, network reachability, secrets, and sandbox class.",
        "Model preferences such as locality, cost, load, data gravity, warm cache, user pinning, and battery/thermal policy.",
        "Produce a human-readable placement explanation and explicit no-eligible-Host diagnosis.",
        "Revalidate capability freshness immediately before dispatch and define migration/retry behavior after Host loss.",
        "Never silently place work on a Host lacking its Role's secret or permission scope."
    ], deps=["H06", "D04", "D05"])
add("H09", "Implement Project-service placement across the Fabric", "E02", 4, "P1", "distributed",
    "Place graph storage, analyzers, embeddings, inference, previews, databases, and platform builds independently from coding task execution.", [
        "Represent service requirements, statefulness, locality, port exposure, persistence, failover, and lifecycle.",
        "Allow explicit placement and policy-driven recommendations for graph, SurrealDB, embedding, local inference, Preview, Windows, and Apple build services.",
        "Prevent two services from becoming simultaneous canonical writers unless the service explicitly supports clustering.",
        "Plan and verify service migration with quiescence, snapshot, transfer, resume, and rollback.",
        "Expose topology and degraded dependencies in clients."
    ], deps=["H07", "H08", "B06"])
add("H10", "Implement distributed task worktree creation and Git-based synchronization", "E02", 4, "P1", "git",
    "Run isolated tasks on remote Hosts without introducing a fragile shared network filesystem.", [
        "Create/fetch the correct commit and task branch on the selected Host and establish a task-owned worktree.",
        "Transfer unpushed prerequisites through an explicit bundle/patch path or block with a clear prerequisite instead of guessing.",
        "Checkpoint, push, fetch, and verify commits at declared handoff boundaries.",
        "Detect remote divergence, missing LFS/submodules, credentials, and repository policy before execution.",
        "Recover or salvage work when a Host disappears mid-task."
    ], deps=["H08", "M02", "M05"])
add("H11", "Implement the transport-neutral Symbiote Link connection framework", "E02", 4, "P0", "network",
    "Use one authenticated Host protocol over local, private, managed, and user-controlled transports.", [
        "Define endpoint providers for loopback/LAN, Tailscale, managed outbound tunnel, custom HTTPS/WSS, and SSH-launched Hosts.",
        "Keep transport selection outside Project and Host identity so changing networks does not create duplicate environments.",
        "Provide reachability probing, preferred-path selection, failover, certificate validation, and clear privacy/cost disclosure.",
        "Never expose an unauthenticated Host RPC endpoint merely because a tunnel is active.",
        "Offer a zero-cloud path for users who choose LAN, tailnet, VPN, SSH, or custom infrastructure."
    ], deps=["H04", "H05"])
add("H12", "Implement controlled recursive improvement and benchmark experiment loops", "E02", 6, "P1", "feature",
    "Allow a specialized Host such as a Fedora Strix Halo inference node to run recursive improvement work under measurable engineering rails.", [
        "Require a baseline, target metric, representative corpus/workload, experiment budget, maximum iterations, and stop conditions.",
        "Run every candidate in isolated issue/task branches and worktrees; never rewrite the canonical checkout in place.",
        "Persist prompts, models, parameters, code changes, benchmark results, resource use, and rejected candidates as artifacts.",
        "Promote only statistically or practically meaningful improvements that pass correctness and regression gates.",
        "Require independent review and a normal PR before integration."
    ], deps=["H10", "K07", "M07", "Q14"])

# ---------------------------------------------------------------------------
# E03 — Harness platform and packs
# ---------------------------------------------------------------------------
add("C01", "Define the Harness Adapter SDK, capability schema, and truthful degradation model", "E03", 1, "P0", "harness-core",
    "Create the single provider-neutral contract through which Symbiote detects, configures, starts, observes, controls, and evaluates coding harnesses.", [
        "Model structured chat, streaming, tools, approvals, user input, resume/fork, models, context limits, usage, rate limits, hooks, MCP, skills, native tasks, images, and worktree safety as negotiated capabilities.",
        "Separate harness type, installed Harness Instance/account, selected model, Role assignment, session, and execution Host.",
        "Require adapters to report unsupported/unknown capability instead of emulating success through brittle parsing.",
        "Define stable normalized events while retaining access to namespaced raw vendor events for debugging.",
        "Version adapter contracts and define compatibility policy for third-party Harness Packs."
    ], deps=["A01", "H02"])
add("C02", "Implement SDK, app-server, RPC/JSONL, ACP, structured-CLI, and generic-PTY transport substrates", "E03", 1, "P0", "harness-core",
    "Avoid one-off process plumbing by providing reusable integration classes with explicit guarantees.", [
        "Implement lifecycle, framing, backpressure, cancellation, heartbeat, stderr/log separation, exit classification, and crash recovery for each substrate.",
        "Define when stdout parsing is authoritative versus advisory and prohibit terminal scraping from claiming structured guarantees.",
        "Support local and remote Host execution through the same adapter interface.",
        "Capture transport/version diagnostics without leaking prompts, credentials, or source by default.",
        "Provide fake harness fixtures that exercise every substrate deterministically."
    ], deps=["C01", "H01"])
add("C03", "Implement harness discovery, installation, versioning, authentication, account instances, and model inventory", "E03", 1, "P0", "harness-core",
    "Let Symbiote understand what is actually available on each Host and maintain multiple isolated authenticated instances per harness.", [
        "Detect executable and SDK versions, supported install channels, update availability, configuration roots, auth state, and health.",
        "Model named instances such as Personal, Work, API, or alternate config homes without mutating HOME globally.",
        "Inventory models, context limits, capabilities, provider/account identity, subscription versus API auth, and stale/unknown metadata.",
        "Require user-visible consent before installing, updating, logging out, or changing an existing harness default.",
        "Provide deterministic diagnostics for missing binary, unsupported version, expired auth, rate limit, and incompatible configuration."
    ], deps=["C01"])
add("C04", "Implement non-destructive native configuration projection and reconciliation", "E03", 1, "P0", "harness-core",
    "Project Symbiote desired state into each harness's native files while preserving user ownership and runtime-specific semantics.", [
        "Create adapter-declared configuration locations, scopes, precedence, merge strategy, managed fields, unknown-field preservation, and reload requirements.",
        "Import existing native configuration into a proposed desired state without assuming Symbiote owns it.",
        "Preview exact file/setting changes, create backups or transactions, apply atomically, verify runtime uptake, and roll back on failure.",
        "Detect drift and distinguish user changes, runtime migrations, unsupported syntax, and Symbiote-managed divergence.",
        "Never flatten harness-specific hooks, commands, prompts, permissions, sessions, or trust into a fictitious universal folder."
    ], deps=["C01", "B04"])
add("C05", "Create the Harness Pack conformance laboratory and compatibility dossier format", "E03", 1, "P0", "testing",
    "Make every advertised harness support level evidence-based and regression-testable.", [
        "Define a dated dossier covering upstream source/version, install/update, config scopes/precedence, trust, auth/accounts, models, sessions, programmatic interface, MCP, skills, instructions, hooks, commands, extensions, approvals, usage, and OS behavior.",
        "Create conformance scenarios for start, turn streaming, tool/approval events, interrupt, resume, crash, config round-trip, drift, worktree, and context/usage reporting.",
        "Grade Native, Deep, Standard, Generic, Experimental, and Unsupported capabilities independently.",
        "Run Linux reference tests and record explicit Windows/macOS status rather than implying portability.",
        "Fail release claims when an adapter's verified upstream range no longer matches the installed version."
    ], deps=["C02", "C03", "C04"])

HARNESS_COMMON = [
    "Produce and commit the complete upstream compatibility dossier before finalizing adapter behavior.",
    "Use the strongest supported structured interface and expose only capabilities demonstrated by conformance tests.",
    "Detect/install/probe/authenticate/list models and support named isolated Harness Instances where upstream permits it.",
    "Reconcile project and user configuration non-destructively, preserving unknown fields and native precedence/trust rules.",
    "Cover start, stream, input/approval, interrupt, resume/fork when supported, cancellation, crash, usage/context, task worktree, and upgrade behavior.",
]

def harness(key, title, wave, priority, outcome, specifics, deps=(), platforms=()):
    add(key, title, "E03", wave, priority, "harness-pack", outcome,
        HARNESS_COMMON + list(specifics), ["C05", *deps], platforms=platforms)

harness("C06", "Implement the Claude Code / Claude Agent SDK Harness Pack", 2, "P0",
        "Provide first-class Claude Code sessions that can serve as Lead or workers without making Claude the owner of Symbiote state.", [
            "Integrate the current Claude Agent SDK/Claude Code programmatic surface, existing login/API authentication, .claude resources, CLAUDE.md, MCP, skills, hooks, commands, permissions, subagents, and session continuity.",
            "Register Symbiote orchestration/context tools as native tools while preserving Role and Project permission boundaries."
        ], platforms=["linux", "windows", "macos"])
harness("C07", "Implement the OpenAI Codex Harness Pack", 2, "P0",
        "Provide first-class Codex sessions through the strongest supported app-server/SDK interface and preserve subscription/API account isolation.", [
            "Research and implement current Codex config homes, AGENTS instructions, skills, MCP, approvals/sandboxing, app-server events, tasks/sessions, model controls, usage, and resume semantics.",
            "Support shared state plus account-specific authentication homes only where upstream semantics are verified."
        ], platforms=["linux", "windows", "macos"])
harness("C08", "Implement the Pi Harness Pack through SDK/RPC", 2, "P0",
        "Use Pi's structured SDK/RPC/JSONL modes, provider breadth, and local-model support as a premier open harness integration.", [
            "Respect .pi/settings.json, .pi extensions/prompts/SYSTEM files, .agents/skills and optional foreign-skill paths, AGENTS.override.md precedence, project trust, packages, model/provider settings, sessions, steering/follow-up queueing, compaction, and server profiles.",
            "Expose extensions and RPC events without treating project-local executable resources as trusted before Pi's trust policy is satisfied."
        ], platforms=["linux", "windows", "macos"])
harness("C09", "Implement the OpenCode Harness Pack", 2, "P0",
        "Provide first-class OpenCode server/SDK sessions and broad provider/model access while preserving its native project configuration.", [
            "Research and support current opencode config, agents, commands, skills, plugins, MCP, permissions, server/session APIs, model/provider inventory, and .agents/.claude skill interoperability.",
            "Use OpenCode's structured server/programmatic interface rather than PTY parsing wherever available."
        ], platforms=["linux", "windows", "macos"])
harness("C10", "Implement a generic Agent Client Protocol (ACP) Harness Pack", 2, "P1",
        "Allow any conforming ACP coding agent to receive deep support without a brand-specific core change.", [
            "Implement ACP initialize/capability negotiation, sessions, prompts, streaming content, tool calls, permission requests, files/terminals where standardized, cancellation, and protocol errors.",
            "Allow a declarative command/profile to instantiate an ACP harness and retain optional vendor extensions in a namespaced channel."
        ], deps=["C02"])
harness("C11", "Implement the Gemini CLI Harness Pack", 3, "P1",
        "Support Gemini CLI as a deeply integrated worker or Lead while honoring its extensive layered configuration and trust semantics.", [
            "Handle system defaults, user, project .gemini/settings.json, system overrides, environment, and CLI precedence; GEMINI.md memory; .gemini/.agents skills; custom TOML commands; MCP; hooks; extensions; policies; plan mode; checkpointing; sandboxing; subagents; and session retention.",
            "Fingerprint or otherwise respect changed project hooks and do not bypass upstream consent/trust through silent projection."
        ], platforms=["linux", "windows", "macos"])
harness("C12", "Implement the Cursor Agent Harness Pack", 3, "P1",
        "Integrate Cursor Agent through ACP or the strongest current structured CLI surface, independent of the Cursor editor UI.", [
            "Research .cursor rules, AGENTS/CLAUDE instruction compatibility, MCP inheritance, auth/account/model behavior, cloud/background agents, approvals, session continuity, and programmatic events.",
            "Clearly distinguish editor-owned features from features available to the standalone Agent CLI."
        ], deps=["C10"], platforms=["linux", "windows", "macos"])
harness("C13", "Implement the GitHub Copilot CLI Harness Pack", 3, "P1",
        "Support Copilot CLI's GitHub identity, instruction merging, native agents, MCP/tooling, and repository-aware workflows.", [
            "Research .github/agents and instruction files, cross-ecosystem instruction loading, MCP, plugins/extensions, permissions, model selection, session/plan behavior, GitHub auth, and structured/headless output.",
            "Avoid confusing GitHub repository authorization with Symbiote's own GitHub integration permission boundary."
        ], platforms=["linux", "windows", "macos"])
harness("C14", "Implement the Kiro CLI Harness Pack", 3, "P1",
        "Support Kiro's project steering, specs, skills, hooks, MCP, agents, permissions, and headless execution without collapsing them into generic skills.", [
            "Research .kiro steering/specs/skills/hooks/agents, user versus project MCP and permissions, auth/model/session behavior, CLI events, trust, and install/update channels.",
            "Preserve Kiro specifications as native artifacts while allowing explicit linkage—not accidental duplication—with Symbiote artifacts."
        ], platforms=["linux", "windows", "macos"])
harness("C15", "Implement the Qwen Code Harness Pack", 3, "P1",
        "Provide a first-class open-model-friendly coding harness integration with truthful current capability detection.", [
            "Research .qwen settings, instruction files, skills/commands/hooks/extensions if supported, MCP, provider/model configuration, auth, headless output, sessions, approvals, and context behavior.",
            "Test both official hosted authentication and user-supplied/local compatible provider paths where upstream supports them."
        ], platforms=["linux", "windows", "macos"])
harness("C16", "Implement the Kimi Code CLI Harness Pack", 3, "P1",
        "Integrate Kimi Code's wire/programmatic modes, background tasks, agents, hooks, MCP, plugins, and multi-provider configuration.", [
            "Respect ~/.kimi config TOML/JSON and --config-file/--config overrides, model context/reserved size, loop control, background tasks, .kimi resources, AGENTS.md, skills merge policy, agents, hooks, plugins, MCP, and wire mode.",
            "Prevent API keys from being copied into portable Project state when projecting provider/model configuration."
        ], platforms=["linux", "windows", "macos"])
harness("C17", "Implement the Factory Droid Harness Pack", 4, "P2",
        "Support Factory Droid's native project/user configuration, commands, skills, hooks, rules, MCP, auth, sessions, and structured execution.", [
            "Document and honor .factory and global precedence, generated versus user-owned files, task/session semantics, install/update channels, and current programmatic interface.",
            "Prove worktree isolation and non-destructive projection through the common conformance suite."
        ], platforms=["linux", "windows", "macos"])
harness("C18", "Implement the Amp Harness Pack", 4, "P2",
        "Support Amp's agent mode, threads, tool permissions, MCP, skills, instructions, and supported programmatic/CLI interfaces.", [
            "Research .amp/user settings, AGENTS.md and .agents/skills behavior, model/auth policy, thread resume/share, commands, MCP, approvals, and usage limits.",
            "Keep Amp provider/account constraints visible to Role routing instead of assuming arbitrary model selection."
        ], platforms=["linux", "windows", "macos"])
harness("C19", "Implement the Grok Build Harness Pack", 4, "P2",
        "Support the current Grok coding CLI/build agent through its best structured interface and native project conventions.", [
            "Research install/auth/model/session/config/MCP/skills/instructions/hooks/approval/headless capabilities from current upstream sources.",
            "Mark unavailable or private interfaces honestly and fall back to structured CLI or PTY with reduced guarantees."
        ], platforms=["linux", "windows", "macos"])
harness("C20", "Implement the Hermes Agent Harness Pack", 4, "P2",
        "Integrate Hermes as a local/open-model-capable worker with its plugins, skills, MCP, checkpoints, memories, gateway, and headless controls.", [
            "Respect HERMES_HOME, ~/.hermes config versus .env secrets, workspace AGENTS.md, memories, hook trust/acceptance, profiles, plugin/tool management, checkpoints, sessions, model providers, and remote gateway behavior.",
            "Do not conflate Hermes long-term memory with canonical Symbiote Project Memory; synchronize only explicit artifacts/facts."
        ], platforms=["linux", "windows", "macos"])
harness("C21", "Implement the Crush Harness Pack", 4, "P2",
        "Support Crush's cross-platform multi-model harness, executable crushrc configuration, LSPs, MCP, sessions, and local models safely.", [
            "Respect project/global crushrc precedence, XDG/Windows paths, executable configuration trust, providers/models, permissions, LSPs, MCP transports, sessions, model switching, and platform-specific behavior.",
            "Never rewrite executable crushrc without an exact preview, user consent, backup, and syntax/behavior verification."
        ], platforms=["linux", "windows", "macos"])
harness("C22", "Implement Aider and Goose standard Harness Packs", 5, "P2",
        "Provide tested standard integrations for widely used CLI agents that may not expose every deep structured capability.", [
            "Maintain separate Aider and Goose dossiers/config projections even when they share the structured-CLI substrate.",
            "Support their native model/provider, instruction, tool/MCP, session, Git, approval, and config semantics without claiming unsupported deep events."
        ], platforms=["linux", "windows", "macos"])
harness("C23", "Implement the user-defined generic CLI/PTTY Harness Pack", 2, "P0",
        "Let users add any local CLI immediately while clearly communicating the reduced reliability of generic integration.", [
            "Allow executable/arguments/environment/cwd, install notes, completion detection, readiness regex, graceful stop, timeout, worktree policy, and optional hooks/parser plugin.",
            "Limit advertised capabilities to process lifecycle, terminal visibility, files/worktree, and explicitly configured signals; never infer structured success from plausible prose output."
        ], deps=["C02"], platforms=["linux", "windows", "macos"])

# ---------------------------------------------------------------------------
# E04 — Team and canonical work
# ---------------------------------------------------------------------------
add("D01", "Implement mandatory per-Project Team and Role configuration", "E04", 2, "P0", "orchestration",
    "Make stable engineering Roles the user-facing staffing abstraction and require enough staffing to execute Project work.", [
        "Require a Lead Orchestrator and at least one general execution Role before autonomous work can begin.",
        "Represent custom Role name, responsibilities, task domains, permissions, context policy, tool/skill policy, execution policy, review requirements, and fallbacks.",
        "Keep Role identity stable when its harness, model, account, or Host changes.",
        "Provide sensible templates without hard-coding a fixed company org chart.",
        "Validate impossible, cyclic, unreviewed, or privilege-escalating Team configurations."
    ], deps=["C01", "B04"])
add("D02", "Implement the primary Lead conversational orchestrator as a selected harness session", "E04", 2, "P0", "orchestration",
    "Let the client talk directly to the configured Lead while Symbiote supplies firm tools and owns state.", [
        "Start/resume the Lead through its assigned Harness Pack and render native streamed content, tool activity, questions, approvals, and errors.",
        "Expose scoped Symbiote tools for requests, tasks, Roles, workers, context, graph, artifacts, Git, review, and status.",
        "Keep the Lead transcript non-authoritative: all durable work must be persisted through canonical tools/events.",
        "Notify/wake the Lead through events instead of token-wasting status polling.",
        "Support voice, desktop, and mobile clients against the same Lead session."
    ], deps=["D01", "D07", "C06", "C07", "C08", "C09"])
add("D03", "Create standard and custom AI-firm Role templates", "E04", 2, "P1", "product",
    "Provide composable staffing patterns for architecture, backend, frontend, mobile, QA, review, security, research, delivery, documentation, and specialized domains.", [
        "Define responsibilities and required capabilities separately from any vendor/model recommendation.",
        "Support multiple Roles of the same type, named specialists, and one harness instance serving several Roles with concurrency limits.",
        "Allow Project-local templates and portable organization templates without copying credentials.",
        "Include independent-review separation and least-privilege defaults.",
        "Show which task classes currently lack an eligible Role."
    ], deps=["D01"])
add("D04", "Implement Role bindings to harness, model, account instance, Host constraints, tools, context, and permissions", "E04", 2, "P0", "orchestration",
    "Resolve each logical Role to a complete executable profile without exposing provider details in normal client prompts.", [
        "Bind primary and fallback Harness Instances/models plus Host requirements/preferences.",
        "Bind MCPs, skills, native resources, secret scopes, filesystem/shell/network/Git permissions, context policy, and concurrency.",
        "Validate model availability/context limits and harness capabilities on eligible Hosts.",
        "Snapshot the resolved assignment on every dispatch for historical provenance.",
        "Explain configuration drift and block dispatch when required capabilities cannot be proven."
    ], deps=["D01", "C03", "N01", "H06"])
add("D05", "Implement semantic work classification and deterministic Role resolution", "E04", 2, "P0", "orchestration",
    "Route architecture, backend, frontend, QA, security, research, release, and custom work by Role rather than model name.", [
        "Allow the Lead to request a Role explicitly and Symbiote to validate eligibility deterministically.",
        "Provide a classifier/recommender for untyped work, but persist the selected semantic reason and confidence.",
        "Use Project domain/graph evidence to split mixed work into coherent Role-owned tasks.",
        "Honor explicit user/Lead assignments and policy before learned performance preferences.",
        "Return a no-route diagnosis with missing Role/capability/Host/account details rather than silently substituting."
    ], deps=["D04", "D07"])
add("D06", "Implement quota, rate-limit, cost, concurrency, and fallback policies", "E04", 3, "P0", "orchestration",
    "Handle exhausted subscriptions and unavailable models without losing task state or making unauthorized quality substitutions.", [
        "Model primary, ordered fallbacks, ask-before-fallback, automatic fallback, queue, and hard-block policies per Role/task risk.",
        "Ingest truthful harness usage/rate-limit signals where available and classify unknown separately from healthy.",
        "Preserve task worktree, artifacts, context handoff, and execution provenance across reassignment.",
        "Require re-verification when a fallback model/harness has lower capabilities or different tool access.",
        "Expose estimated/actual cost and token/resource budgets without requiring Symbiote inference resale."
    ], deps=["D04", "D08", "C03"])
add("D07", "Implement Client Request, Objective, Capability, Plan, Task DAG, Dispatch, and Artifact domain objects", "E04", 1, "P0", "domain",
    "Create the canonical work hierarchy that replaces provider-local todo lists as Project truth.", [
        "Define immutable IDs, versioned schemas, ownership, states, dependencies, provenance, timestamps, priority, constraints, acceptance, evidence, approvals, and relationships.",
        "Separate a client's outcome request, a measurable capability, the plan, executable tasks, individual dispatch attempts, and runtime-local subtasks.",
        "Define legal state transitions and idempotent commands/events for every object.",
        "Persist foreign runtime session/task references without granting them authority over canonical status.",
        "Support parent/child and cross-Project references without creating hidden dependency cycles."
    ], deps=["B01", "H02"])
add("D08", "Implement task dispatch, worker lifecycle, and runtime-local task mapping", "E04", 2, "P0", "orchestration",
    "Execute a canonical task through one or more tracked dispatch attempts and reconcile runtime events without confusing local todos with global work.", [
        "Resolve Role, harness/model/account, Host, worktree, context bundle, permissions, tools, and verification contract before launch.",
        "Track queued, starting, running, waiting-input, blocked, retrying, completed, failed, cancelled, superseded, and orphaned states.",
        "Store runtime session/local-task IDs as foreign references and ingest local progress only as advisory evidence unless promoted.",
        "Support interrupt, steer, cancel, resume, reassign, retry, and salvage with explicit side effects.",
        "Prevent a worker from marking canonical completion without Symbiote's completion gates."
    ], deps=["D05", "D07", "C02", "M02", "I01"])
add("D09", "Implement event-driven Lead notifications, worker questions, approvals, and client escalation policy", "E04", 2, "P0", "orchestration",
    "Move information to the right decision-maker without inference polling or burdening the client with routine implementation choices.", [
        "Define task-complete, failed, blocked, needs-input, verification-failed, conflict, dependency-ready, quota, Host-loss, and approval events.",
        "Route routine technical questions to the Lead or configured specialist; escalate product, cost, risk, destructive, or irreversible decisions according to policy.",
        "Deduplicate/coalesce noisy events and preserve urgency, cause, deadline, and affected objects.",
        "Synchronize unanswered questions and approvals across all clients with exactly-once resolution.",
        "Persist the decision and downstream impact, not merely the chat answer."
    ], deps=["D02", "D08", "H03"])
add("D10", "Implement global Task Graph status, progress, blocking, and health calculations", "E04", 2, "P0", "feature",
    "Give clients and the Lead one evidence-based view of current goals, tasks, workers, blockers, and remaining closure.", [
        "Calculate progress from completed weighted work and closure evidence rather than model self-reported percentages alone.",
        "Expose dependency paths, critical path, blocked reasons, waiting Roles, active dispatches, unresolved impact, diagnostics, tests, PRs, and approvals.",
        "Support filters by Objective, Capability, Role, Host, status, issue, PR, and time.",
        "Define stale/unknown state and never display it as healthy or complete.",
        "Provide efficient subscription APIs for desktop and mobile right-side control surfaces."
    ], deps=["D07", "D08", "F07"])
add("D11", "Implement Lead orchestrator handoff and mid-objective replacement", "E04", 4, "P1", "orchestration",
    "Allow a user to switch the conversational brain without losing the engineering firm's state.", [
        "Build a compact handoff from objectives, task graph, decisions, active dispatches, artifacts, blockers, constraints, and recent relevant conversation—not a blind transcript dump.",
        "Pause or fence concurrent Lead writes during authority transfer.",
        "Require the incoming Lead to acknowledge unresolved decisions and active-risk items before resuming orchestration.",
        "Preserve old session provenance and allow rollback to the previous Lead where safe.",
        "Test quota exhaustion, crash, user-requested switch, and incompatible-capability cases."
    ], deps=["D02", "I04", "F01"])
add("D12", "Implement routing explainability, manual override, and evidence-based performance profiles", "E04", 6, "P1", "orchestration",
    "Make routing understandable and adaptable without hard-coding model folklore.", [
        "Explain semantic Role choice, resolved harness/model/account, Host placement, fallbacks, permissions, context budget, and policy inputs.",
        "Allow bounded manual override before or during dispatch with impact warnings and provenance.",
        "Record task-type success, verification failures, latency, cost, context efficiency, review findings, and regressions by profile.",
        "Use performance history only inside user-authorized routing modes and never silently rewrite explicit Team configuration.",
        "Provide export/reset and bias/privacy controls for local performance data."
    ], deps=["D05", "D06", "H08", "K10"])

# ---------------------------------------------------------------------------
# E05 — Sessions and artifacts
# ---------------------------------------------------------------------------
add("F01", "Implement canonical Project/Role/session identities across Harnesses and clients", "E05", 2, "P0", "domain",
    "Keep Lead and worker sessions durable, resumable, searchable, and independent from the GUI that opened them.", [
        "Separate canonical Session, Harness Session, Dispatch Attempt, client view, and transcript branch/fork identities.",
        "Persist normalized events plus pointers to raw runtime records when available.",
        "Define resume, fork, compact, archive, retention, export, deletion, and orphan recovery.",
        "Organize sessions by Objective/Task/Role rather than an unstructured chronological sidebar.",
        "Prevent one Project's session discovery from leaking into another Project."
    ], deps=["D07", "H03", "C01"])
add("F02", "Build the live Agent Session inspector and steering surface", "E05", 4, "P1", "feature",
    "Let users inspect any worker's reasoning-visible events, tool activity, terminal output, context, diffs, questions, and state without making sessions the primary product model.", [
        "Render normalized and optional raw vendor events with timestamps and correlation to tasks/artifacts/files.",
        "Support steer/follow-up/interrupt/cancel only when the Harness Pack truthfully supports them.",
        "Expose context budget, compaction, usage, model, Host, worktree, permissions, and tools.",
        "Provide redaction and secret-safe export.",
        "Handle very large streams through virtualization, pagination, and bounded retention."
    ], deps=["F01", "D08", "I06"])
add("F03", "Define and implement the first-class Artifact schema and type registry", "E05", 2, "P0", "domain",
    "Represent plans, specs, audits, decisions, research, reviews, test reports, wireframes, screenshots, annotations, benchmarks, and delivery evidence as durable work products.", [
        "Define type/version, status, producer Role/dispatch, source inputs, Objective/Capability/Task, graph nodes, files, requirements, provenance, sensitivity, and storage representation.",
        "Support structured data plus human-readable rendered views and attached binary assets.",
        "Distinguish repository files from engineering artifacts while allowing an artifact to materialize/export into the repository.",
        "Provide immutable version identities and explicit supersession rather than destructive overwrite.",
        "Index artifacts for retrieval without treating their entire bodies as mandatory context."
    ], deps=["D07", "B01"])
add("F04", "Implement artifact versions, comparison, approval, rejection, and locked baselines", "E05", 3, "P1", "feature",
    "Make evolving specifications and reviews auditable and safe for downstream dependency use.", [
        "Create drafts, revisions, diffs, approval states, approvers, rejection reasons, locks, and supersession chains.",
        "Show semantic and textual differences with affected requirements/decisions/tasks.",
        "Prevent downstream readiness from silently following an unapproved or superseded artifact.",
        "Allow amendments through a new version with propagation impact rather than in-place mutation.",
        "Support client and Role approval policies."
    ], deps=["F03", "J08"])
add("F05", "Implement artifact comments, anchored annotations, ownership, and delta notifications", "E05", 4, "P1", "feature",
    "Provide Traycer/Figma-like collaboration without re-injecting whole changed artifacts into every agent context.", [
        "Anchor comments to structured nodes, document ranges, DOM targets, image regions, graph nodes, files, diffs, or artifact sections with fallback anchors.",
        "Track open/resolved/outdated threads, authors, mentions, decisions, and resulting tasks.",
        "Assign one canonical writer or merge policy per artifact version and detect concurrent edits.",
        "Notify dependent agents with structured deltas and affected sections, not full artifact bodies by default.",
        "Preserve comments across revisions where anchors still map and mark ambiguous remaps for review."
    ], deps=["F04", "H03", "I04"])
add("F06", "Implement cross-artifact dependencies, validation, and spec-to-task derivation", "E05", 3, "P1", "feature",
    "Turn artifacts into a coherent engineering model rather than a folder of nice-looking documents.", [
        "Define derives-from, depends-on, contradicts, supersedes, implements, verifies, and blocks relationships.",
        "Validate cross-artifact references, decisions, requirements, contracts, and status compatibility.",
        "Generate proposed capabilities/tasks from approved specifications with traceability and explicit review.",
        "Flag artifact changes that invalidate downstream plans, tasks, tests, or active PR assumptions.",
        "Expose unresolved cross-artifact errors in readiness and ambiguity health."
    ], deps=["F03", "G11", "J09"])
add("F07", "Implement the immutable activity, audit, and provenance timeline", "E05", 2, "P0", "feature",
    "Provide an intelligible chronological record of what the firm, clients, Hosts, and integrations did and why.", [
        "Record domain events for requests, decisions, assignments, context packages, file/Git changes, diagnostics, reviews, approvals, deployment, and security-sensitive actions.",
        "Correlate events across Objective, Task, Dispatch, Role, Harness, Host, worktree, PR, and Artifact.",
        "Support filtering, pagination, retention classes, export, redaction, and tamper-evident integrity for high-risk events.",
        "Keep verbose raw process logs separate while linking them by correlation ID.",
        "Render concise human activity summaries without using an LLM as the source of truth."
    ], deps=["H02", "D07"])

# ---------------------------------------------------------------------------
# E06 — System Graph foundation
# ---------------------------------------------------------------------------
add("G01", "Define the multi-layer System Graph ontology, provenance, confidence, and source-range model", "E06", 3, "P0", "graph",
    "Create the canonical digital-twin schema before analyzers produce incompatible edge vocabularies.", [
        "Model product, requirement, decision, architecture, repository, file, syntax, symbol, type, call, control/data flow, schema, API, event, config, infrastructure, test, runtime, Git, task, artifact, and release nodes.",
        "Define typed directed edges, validity intervals, analyzer/source, confidence, conditions, environment, commit, and exact byte/line provenance.",
        "Support confirmed, probable, possible, observed, inferred, stale, unresolved, and contradicted evidence without collapsing certainty.",
        "Define semantic identity and aliasing across languages, generated code, repositories, serialized names, and versions.",
        "Publish ontology migrations and query compatibility tests."
    ], deps=["A01", "D07", "F03"])
add("G02", "Design and implement the SurrealDB 3+ graph/vector/full-text storage layer", "E06", 3, "P0", "graph",
    "Use SurrealDB as a persistent multimodel substrate while preserving replaceable analyzer and storage boundaries.", [
        "Define records/relations, indexes, namespace/database isolation, migrations, transactions, batch ingestion, query limits, and backup/restore.",
        "Support graph traversal, BM25/full text, vectors, metadata filtering, and hybrid candidate fusion without duplicating canonical identity.",
        "Benchmark representative small, monorepo, and very large Project datasets for ingest, incremental update, traversal, hybrid search, and memory/disk behavior.",
        "Plan version pinning and compatibility for shared and isolated local profiles.",
        "Expose health and graceful degradation when vector/reranker services are unavailable."
    ], deps=["G01", "B06"])
add("G03", "Implement analyzer/indexer plugin contracts and normalized evidence ingestion", "E06", 3, "P0", "graph",
    "Aggregate best-of-breed compiler, LSP, parser, security, Git, runtime, and framework evidence instead of writing one mediocre universal parser.", [
        "Define analyzer discovery, language/framework applicability, input commit/root, emitted nodes/edges, confidence, diagnostics, incremental invalidation, cancellation, and version metadata.",
        "Deduplicate or reconcile complementary/contradictory evidence without deleting provenance.",
        "Sandbox or permission analyzers according to their code execution and source access.",
        "Provide deterministic fixture graphs and schema validation.",
        "Allow third-party analyzers without granting direct unrestricted database mutation."
    ], deps=["G01", "G02", "N11"])
add("G04", "Index repositories, files, exact ranges, syntax trees, declarations, and symbols", "E06", 3, "P0", "graph",
    "Create byte-precise source attribution and syntax structure for every supported Project root.", [
        "Ingest repository/root/file/language/generated/ignored metadata and content hashes.",
        "Use Tree-sitter or compiler parsers to model declarations, statements, expressions, parameters, literals, imports, and syntax parent/child structure.",
        "Map one-character diffs to affected semantic nodes without representing each byte as a graph vertex.",
        "Handle parse errors, partial files, generated code, vendored dependencies, symlinks, submodules, and very large files explicitly.",
        "Provide initial language coverage and a tested extension path."
    ], deps=["G03", "B03"])
add("G05", "Ingest compiler, LSP, SCIP, type, definition, reference, implementation, and import evidence", "E06", 3, "P0", "graph",
    "Add resolved semantic relationships that syntax parsing alone cannot provide.", [
        "Normalize definitions, references, types, overrides, implements/extends, imports/exports, dynamic dispatch candidates, and cross-package identities.",
        "Prefer compiler-backed evidence while retaining heuristic and SCIP evidence with provenance when compiler truth is unavailable.",
        "Manage per-Project LSP lifecycle, workspace roots, initialization, progress, crashes, and versioned caches.",
        "Represent unresolved symbols and ambiguous targets as first-class evidence gaps.",
        "Test cross-file, cross-package, cross-repository, and generated-type relationships."
    ], deps=["G04"])
add("G06", "Build call, control-flow, data-flow, alias, and taint-capable graph overlays", "E06", 4, "P1", "graph",
    "Model causal value and execution relationships required for real blast radius and security analysis.", [
        "Produce direct and possible call edges, CFG blocks/branches, value/data flow, parameter/return propagation, aliases, sources, sinks, and sanitizers where supported.",
        "Support intraprocedural and bounded interprocedural analysis with explicit truncation and confidence.",
        "Keep static possibility separate from runtime observation.",
        "Integrate or translate evidence from proven tools such as CodeQL/CPG-style analyzers where licensing/deployment permits.",
        "Expose unresolved dynamic/reflection boundaries rather than falsely declaring full closure."
    ], deps=["G05"])
add("G07", "Index databases, schemas, API contracts, events, configuration, feature flags, and serialization identities", "E06", 3, "P0", "graph",
    "Connect code symbols to the contracts and data representations most likely to cause obscure cross-surface breakage.", [
        "Model tables/columns/indexes/policies/migrations, request/response schemas, routes, GraphQL/OpenAPI, queues/events/topics, environment/config values, flags, DTOs, and generated clients.",
        "Link reads/writes/validates/serializes/deserializes/emits/consumes/guards/generates relationships.",
        "Represent one semantic field across snake_case, camelCase, language types, SQL, analytics, and wire formats.",
        "Provide framework analyzers for initial target stacks and a generic declarative extractor.",
        "Detect stale generated code and contract-version mismatches."
    ], deps=["G04", "G05"])
add("G08", "Ingest tests, assertions, fixtures, coverage, and runtime execution traces", "E06", 4, "P1", "graph",
    "Connect implementation nodes to actual behavioral verification and observed execution paths.", [
        "Model test suites/cases, assertions, fixtures, mocks, environments, covers/exercises/protects relationships, results, and flaky history.",
        "Import line/branch/function coverage and map it to semantic nodes at the tested commit.",
        "Accept runtime traces/telemetry with environment and observation counts while preserving static possibilities.",
        "Separate test-only reachability from production entrypoint reachability.",
        "Prevent stale coverage from being used as current completion evidence."
    ], deps=["G04", "G06", "Q04"])
add("G09", "Index infrastructure, deployment, service, queue, storage, DNS, and release topology", "E06", 4, "P1", "graph",
    "Extend code awareness into the systems that build, host, connect, and distribute the product.", [
        "Model deployable services, containers, functions, workers, queues, databases, object stores, caches, networks, gateways, DNS/TLS, secrets references, environments, build artifacts, stores, and release channels.",
        "Link source/config/infra definitions to built and deployed artifacts and runtime dependencies.",
        "Ingest IaC, compose, workflow, platform config, manifests, and delivery artifacts with provenance.",
        "Represent public versus internal exposure, region, environment, version, and health.",
        "Support delivery blast-radius and rollback queries."
    ], deps=["G07", "K06"])
add("G10", "Index Git commits, diffs, blame, co-change, regressions, and temporal graph history", "E06", 4, "P1", "graph",
    "Add temporal and empirical relationships that static structure misses.", [
        "Map commits/PRs/issues/authors/timestamps/diffs to changed semantic nodes and contracts.",
        "Compute co-change relationships with sample size, recency, repository boundary, and confidence rather than raw coincidence.",
        "Preserve graph validity by commit and support before/after queries for regressions and architecture drift.",
        "Link failing tests/incidents to introducing and fixing changes where evidence exists.",
        "Avoid exposing sensitive authorship or private remote metadata outside Project policy."
    ], deps=["G04", "M01", "M06"])
add("G11", "Link product requirements, decisions, artifacts, issues, tasks, code, tests, PRs, deployments, and releases", "E06", 3, "P0", "graph",
    "Make end-to-end why/what/how/verified traceability a core graph layer.", [
        "Define derives-from, specifies, constrains, implements, modifies, tests, reviews, approves, deploys, supersedes, and verifies relationships.",
        "Require every material task and PR to point to a Capability/requirement or an explicit maintenance rationale.",
        "Support requirement-to-implementation closure and code-to-original-decision explanation.",
        "Record agent/human/Host provenance without treating model prose as evidence of implementation.",
        "Detect orphan requirements, unplanned code, stale specs, and release evidence gaps."
    ], deps=["G01", "F06", "M04"])
add("G12", "Implement incremental indexing, invalidation, Project suspension, and consistency repair", "E06", 3, "P0", "graph",
    "Keep the System Graph current during multi-agent and human edits without full reindexing or stale certainty.", [
        "Use content hashes, Git diffs, parser dependencies, generated artifacts, config/lockfile changes, and analyzer declarations to compute invalidation.",
        "Debounce/coalesce file events but guarantee eventual convergence at a named commit/worktree state.",
        "Pause/drain/checkpoint/resume analyzers under Project lifecycle and Host migration.",
        "Detect interrupted ingestion, missing edges, analyzer version changes, schema drift, and stale snapshots; repair idempotently.",
        "Expose indexing lag and never present stale impact closure as current."
    ], deps=["G02", "G03", "H01"])
add("G13", "Implement vector embeddings, BM25, fuzzy/symbol search, candidate fusion, and reranking", "E06", 3, "P0", "graph",
    "Provide complementary lexical and semantic entry points into the System Graph without reducing code understanding to chunk RAG.", [
        "Index semantic entities/artifacts and appropriately bounded source chunks with model/version/dimension provenance.",
        "Support exact/symbol/fuzzy/path/BM25/vector queries plus graph-derived candidates.",
        "Implement configurable fusion and reranking with filters, diversity, recency, and token cost.",
        "Allow local and remote embedding/reranking services through service placement and privacy policy.",
        "Create relevance, latency, memory, and stale-index evaluation corpora."
    ], deps=["G02", "G04", "H09"])
add("G14", "Implement the System Graph query API, saved queries, explorer, and evidence explanations", "E06", 4, "P1", "feature",
    "Make graph intelligence available to clients, agents, policies, and power users without unrestricted raw-database coupling.", [
        "Provide typed traversal/search/query plans, limits, pagination, cancellation, explain, and result provenance.",
        "Offer safe parameterized queries and permission-filtered projections; prohibit arbitrary write access from agents.",
        "Support saved queries such as dead code, all writers, affected consumers, untested paths, and architecture violations.",
        "Visualize filtered subgraphs, paths, edge certainty, time/commit, and why each result exists.",
        "Export results as structured artifacts for review and task creation."
    ], deps=["G01", "G02", "G13"])

# ---------------------------------------------------------------------------
# E07 — Context and graph applications
# ---------------------------------------------------------------------------
add("I01", "Define the Context Broker model, budgets, reservations, and delivery contracts", "E07", 3, "P0", "context",
    "Treat context windows as bounded working memory, not targets to fill or transcripts to dump.", [
        "Model model window, system/harness overhead, tool schemas, output reserve, working headroom, initial injection, dynamic expansion, and compaction reserve.",
        "Define mandatory task contract, direct source, structural graph, semantic/lexical, project knowledge, historical artifacts, and raw-history tiers.",
        "Support Role/Project/model policies and hard maximums while allowing useful context below the ceiling.",
        "Represent every included fragment as a reference with provenance, relevance reason, sensitivity, token count, and freshness.",
        "Block dispatch when the mandatory contract cannot fit rather than silently truncating critical constraints."
    ], deps=["G13", "D07", "C03"])
add("I02", "Implement task-, Role-, model-, permission-, and Host-aware retrieval planning", "E07", 3, "P0", "context",
    "Select context based on what work is being performed, who performs it, available tools, and proven capacity.", [
        "Derive retrieval targets from task contract, affected graph concepts, Role specialization, expected edits, dependencies, and verification contract.",
        "Filter source, secrets, artifacts, environments, and graph nodes by Role/Host/client permission before ranking.",
        "Use actual model context capabilities and harness overhead from the resolved Harness Instance.",
        "Choose conservative, automatic, and deep strategies as policies/ceilings rather than filler percentages.",
        "Persist the plan and allow the Lead/reviewer to inspect why it was chosen."
    ], deps=["I01", "D04", "H08"])
add("I03", "Implement graph-structural retrieval fused with semantic, lexical, fuzzy, LSP, and history evidence", "E07", 3, "P0", "context",
    "Find the causally and structurally relevant system neighborhood, then augment it with conceptual knowledge.", [
        "Use target symbols/contracts/workflows as graph entry points and traverse callers, callees, data, schemas, events, tests, infrastructure, and cross-repo paths under bounded rules.",
        "Use vector/BM25/fuzzy/search to discover conceptual nodes, ADRs, requirements, and exact identifiers missed by structural traversal.",
        "Fuse/rerank candidates using graph distance/type/confidence, semantic/lexical score, task role, recency, impact, and token cost.",
        "Avoid naive concatenation and duplicate code/artifact inclusion.",
        "Evaluate retrieval against expert-curated task context and regression scenarios."
    ], deps=["I02", "G14"])
add("I04", "Implement structured Context Bundles, dependency handoffs, and delta-only cross-agent communication", "E07", 3, "P0", "context",
    "Transfer exact task state and artifacts instead of forcing agents to reread other agents' full conversations.", [
        "Package objective/task/acceptance, constraints, decisions, relevant graph references/source ranges, dependency contracts, artifacts, permissions, tools, and verification.",
        "Define producer handoff summaries with interfaces, changed assumptions, files/symbols, commits, evidence, open questions, and downstream obligations.",
        "Send artifact/source deltas and affected references after updates rather than reinjecting full documents.",
        "Version bundles and retain which bundle produced each dispatch/result.",
        "Reject stale handoffs when prerequisite commits/artifacts have changed beyond declared compatibility."
    ], deps=["I03", "F03", "D08"])
add("I05", "Implement just-in-time context expansion, compaction, continuation, and recovery", "E07", 4, "P1", "context",
    "Start lean, let workers request targeted additions, and preserve durable work across compaction or model replacement.", [
        "Expose a scoped context-expand tool requiring query, reason, desired evidence type, and bounded token request.",
        "Deduplicate against existing bundle and return references/deltas with updated budget.",
        "Extract durable decisions, findings, artifacts, and task state before context compaction or session handoff.",
        "Treat worker summaries as claims linked to evidence, not automatic Project truth.",
        "Test long-running tasks, repeated expansion, quota switch, crash, and 128K versus large-window models."
    ], deps=["I04", "F01", "D11"])
add("I06", "Build the context inspector, token ledger, and ‘why included’ explanations", "E07", 4, "P1", "feature",
    "Make context quality and cost observable rather than magical.", [
        "Show mandatory, code, graph, contracts, decisions, artifacts, tests, history, tool/system, output reserve, and headroom token categories.",
        "Explain target relationship, graph path, search/rerank score, dependency, policy, freshness, and sensitivity for each item.",
        "Allow authorized users to remove, pin, or request more context before dispatch with impact warnings.",
        "Compare planned versus actual context/usage/compaction and record waste or missing-context incidents.",
        "Redact secret values while still showing that a secret capability was supplied."
    ], deps=["I01", "I03", "F02"])
add("I07", "Implement pre-edit and post-diff blast-radius analysis", "E07", 4, "P0", "graph-app",
    "Predict and then verify every known place a proposed code, schema, event, config, or architectural change can affect.", [
        "Map a requested/proposed diff to changed semantic nodes and traverse typed reverse/forward dependencies with certainty tiers.",
        "Group direct, transitive, cross-repo, runtime-observed, co-change, contract, data, event, infrastructure, test, and documentation impact.",
        "Compare predicted impact before editing with actual changed nodes after editing and flag unexpected expansion.",
        "Produce a reviewable Impact Analysis artifact with unresolved dynamic boundaries.",
        "Meet measured precision/recall targets on seeded change scenarios."
    ], deps=["G06", "G07", "G08", "G09", "G10", "G14"])
add("I08", "Implement dead code, orphan subsystem, stale flag, and unreachable-path queries", "E07", 4, "P1", "graph-app",
    "Turn dead-code discovery into explainable graph reachability rather than grep-based deletion guesses.", [
        "Define active entrypoints by environment and compute reachable closures across static, framework registration, reflection/plugin, event, config, and observed runtime edges.",
        "Classify high-confidence dead, probably dead, possibly dynamic, test-only, deprecated-only, flag-disabled, and orphan subsystem candidates.",
        "Show incoming/outgoing evidence, runtime observation window, public exports, and uncertainty before deletion.",
        "Generate cleanup tasks with required affected tests and rollback plan instead of auto-deleting by default.",
        "Evaluate false positives against dynamic/plugin-heavy fixtures."
    ], deps=["G06", "G07", "G08", "G14"])
add("I09", "Implement graph-derived affected-test selection and verification coverage", "E07", 4, "P0", "graph-app",
    "Run the smallest defensible fast loop while proving which affected graph remains unverified.", [
        "Map changed/impacted nodes to tests through direct references, coverage, assertions, contracts, co-change, and historical regression evidence.",
        "Order smoke/unit/integration/E2E and platform checks by risk, cost, and dependency.",
        "Report selected tests, omitted suites, rationale, affected graph coverage, stale coverage, and unverified nodes.",
        "Escalate from targeted to broader suites when failures, low confidence, or high-risk boundaries warrant it.",
        "Compare selector misses against full-suite seeded regressions."
    ], deps=["I07", "G08", "Q04"])
add("I10", "Implement graph-defined architecture, security, privacy, and data-boundary policies", "E07", 5, "P1", "graph-app",
    "Make architectural and security constraints executable queries rather than forgotten prose.", [
        "Define allowed/forbidden layer dependencies, privileged paths, unauthenticated reachability, PII flows, external sinks, secret use, and service exposure rules.",
        "Run policies incrementally on diffs and comprehensively at review/release gates.",
        "Show the exact violating path and supporting edges with uncertainty.",
        "Support justified, scoped, expiring suppressions linked to decisions and reviews.",
        "Prevent low-confidence absence of evidence from being represented as proof of safety."
    ], deps=["G06", "G07", "G09", "Q01"])
add("I11", "Implement graph-aware parallel-task overlap prediction and scheduling constraints", "E07", 5, "P1", "graph-app",
    "Prevent agents from colliding semantically even when they edit different files.", [
        "Predict each task's affected subgraph from its contract and planned changes.",
        "Compute symbol/contract/schema/event/test/infrastructure overlap and classify merge/integration risk.",
        "Recommend serialize, establish interface contract first, partition ownership, or isolate with planned synthesis.",
        "Recalculate overlap when actual diffs expand and notify owners before destructive divergence.",
        "Feed decisions to worktree, task DAG, merge order, and context handoff."
    ], deps=["I07", "D10", "M02"])
add("I12", "Implement Impact Closure states and block premature task/capability completion", "E07", 5, "P0", "graph-app",
    "Require every material predicted consequence to be modified, verified unaffected, deferred, or explicitly suppressed before completion.", [
        "Create UNREVIEWED, REQUIRES_CHANGE, MODIFIED, VERIFIED_UNAFFECTED, DEFERRED, SUPPRESSED_WITH_JUSTIFICATION, and FAILED_VERIFICATION states.",
        "Attach owner, evidence, commit, reviewer, reason, validity, and affected node/path to every resolution.",
        "Automatically add newly discovered impact without losing prior resolution history.",
        "Block canonical task/capability completion when policy-significant nodes remain unresolved.",
        "Expose closure progress in Lead, Agent Floor, task sidebar, PR review, and release evidence."
    ], deps=["I07", "I09", "D10", "K08"])

# ---------------------------------------------------------------------------
# E08 — Deep Guidance
# ---------------------------------------------------------------------------
add("J01", "Implement the native Deep Guidance state machine and readiness dependency graph", "E08", 3, "P0", "guidance",
    "Replace CFSA's skill-chain ceremony with enforceable software states, dependencies, gates, and persisted progress.", [
        "Model discovery, vision, product/UX, architecture, delivery, contracts, planning, implementation readiness, closure, and production readiness as dependency-driven states.",
        "Define entry/exit conditions, required evidence, permitted decision types, blocking ambiguity, artifacts, and downstream invalidation.",
        "Allow independent domains to progress when their upstream dependencies are ready without bypassing unresolved constraints.",
        "Persist checkpoints/resume and make every auto-confirmed gate produce the same evidence trail as interactive mode.",
        "Prevent a selected Lead from skipping native gates through prompt claims."
    ], deps=["A01", "D07", "F03"])
add("J02", "Implement adaptive idea/input ingestion from voice, text, documents, transcripts, designs, and existing repositories", "E08", 3, "P0", "guidance",
    "Accept the messy material clients actually have and extract signal without discarding detail.", [
        "Classify one-liner/verbal, thin structured idea, rich specification, noisy conversation transcript, existing product/repository, and mixed corpus inputs.",
        "Preserve source attribution for extracted facts/decisions and distinguish confirmed, inferred, proposed, conflicting, and missing information.",
        "Remove conversational noise without collapsing repeated emphasis or rejected alternatives.",
        "Detect existing downstream artifacts and produce an invalidation/merge plan before overwrite.",
        "Support incremental additions and resume without rereading every source into the Lead context."
    ], deps=["J01", "F03", "I03"])
add("J03", "Implement recursive breadth-before-depth product exploration and fractal Product Graph growth", "E08", 3, "P0", "guidance",
    "Map the whole product before overdeveloping the first interesting feature, then deepen every leaf reactively.", [
        "Classify project shape, surfaces, audiences, domains, subdomains, features, workflows, constraints, and technical concerns before placement.",
        "Run global domain map, breadth sweep, then recursive leaf deepening with explicit DEFINED/DEEP/EXHAUSTED coverage semantics.",
        "Promote a feature to a subdomain when internal interacting capabilities emerge while preserving stable identity and references.",
        "Store structured nodes/edges as source of truth and render documents as views/exports.",
        "Detect shallow or missing leaves and block premature vision completion."
    ], deps=["J02", "G01"])
add("J04", "Implement problem, persona, role-lens, workflow, why-now, constraint, and success-metric discovery", "E08", 3, "P0", "guidance",
    "Ground features in real actors, end-to-end outcomes, present alternatives, constraints, and measurable value.", [
        "Require complete persona definitions and map access/behavior/needs across every relevant feature.",
        "Trace each persona's end-to-end workflow including discovery, onboarding, failure, recovery, support, administration, and exit.",
        "Capture problem, current workaround, why now, business/product constraints, risks, success and anti-metrics.",
        "Surface conflicting persona goals and cross-surface handoffs as explicit decisions/cross-cuts.",
        "Reject feature catalogs with no coherent user outcome or Must Have path."
    ], deps=["J03"])
add("J05", "Implement current competitor discovery, capability extraction, and atomic parity/gap audits inside Deep Guidance", "E08", 3, "P0", "guidance",
    "Give every new or evolving product the kind of exhaustive competitor analysis used to shape RYT.", [
        "Discover direct, adjacent, substitute, open-source, platform, and emerging competitors using current evidence.",
        "Audit capabilities atomically within complete persona workflows, surfaces, roles, integrations, migration, operations, pricing, and delivery.",
        "Distinguish required parity, strategic differentiation, deliberate omission, target mismatch, and unsupported competitor claim.",
        "Generate a dated Competitor Matrix artifact and proposed requirements linked to source evidence.",
        "Require client/Lead disposition for material gaps and remember rejected items with rationale."
    ], deps=["J04", "A02", "A03"])
add("J06", "Implement adjacent-feature, standard-category, unmet-persona, and workflow-escape analysis", "E08", 3, "P0", "guidance",
    "Systematically find what the client did not know to ask for.", [
        "Compare the product graph against expected categories for its product/business type.",
        "Ask what each persona still cannot accomplish and where users must leave the product to finish a job.",
        "Compare accepted competitor capability categories to the feature graph.",
        "Present additions as evidence-backed suggestions with add, reject, discuss, or defer decisions.",
        "Persist considered/rejected rationale to prevent repetitive AI suggestions."
    ], deps=["J04", "J05"])
add("J07", "Implement cross-cutting concern and emergent-capability synthesis", "E08", 3, "P0", "guidance",
    "Discover requirements and advantages created by interactions between features, domains, roles, and surfaces.", [
        "Maintain hierarchical cross-cut ledgers as graph relationships rather than isolated markdown files.",
        "Evaluate at least Must×Must and Must×Should feature interactions for new capability, conflict, shared state, policy, or handoff.",
        "Trace workflows crossing domains/surfaces and define contracts at every handoff.",
        "Classify findings as already captured, cross-cut requirement, new feature, constraint, or ambiguity.",
        "Block multi-domain exhaustion when zero credible cross-cuts were evaluated."
    ], deps=["J03", "J06"])
add("J08", "Implement decision authority, classification, ledger, locking, rationale, and supersession", "E08", 3, "P0", "guidance",
    "Persist who may decide product, architecture, implementation, risk, cost, and operational questions and propagate consequences.", [
        "Classify client/product, architecture/options, implementation, security/compliance, delivery, and temporary experiment decisions.",
        "Record question, alternatives, selected answer, authority, rationale, evidence, scope, assumptions, dependencies, status, and affected graph nodes.",
        "Lock approved upstream decisions and prevent downstream contradiction without an explicit amendment/supersession workflow.",
        "Detect conflicts with existing decisions during conversation, artifact edits, planning, and code review.",
        "Render decision history and current effective value at any graph node."
    ], deps=["J01", "G11", "A04"])
add("J09", "Implement ambiguity objects, coverage rubrics, blockers, and specification health", "E08", 3, "P0", "guidance",
    "Turn ambiguity into evidence-backed tracked work instead of a vague quality score.", [
        "Represent missing behavior, contradiction, underspecified state, implicit assumption, cross-layer mismatch, unresolved boundary, and known open question separately.",
        "Link every ambiguity to exact artifact/graph source, affected implementers, severity, required authority, and downstream blockers.",
        "Compute health by product, architecture, interaction, contract, delivery, and implementation layers without averaging blockers away.",
        "Require evidence for both pass and fail findings.",
        "Show zero blocking ambiguity as a gate, not a promise that every future question is known."
    ], deps=["J08", "F06"])
add("J10", "Implement adversarial ambiguity auditing with implementation simulation and the two-implementer test", "E08", 3, "P0", "guidance",
    "Find every decision a developer would otherwise invent and every apparently complete statement that fails under independent interpretation.", [
        "Audit one coherent source unit at a time and cite exact evidence for findings.",
        "Simulate a stub implementation/operation and record every unprovided decision unconditionally.",
        "Ask two isolated implementer/reviewer Roles to describe what they would build and compare divergent assumptions.",
        "Run devil's-advocate, junior-developer, malicious-implementer, failure-mode, cross-layer, and downstream-consistency passes.",
        "Distinguish explicit known unknowns from accidental ambiguity while still blocking work that depends on them."
    ], deps=["J09", "D03", "I04"])
add("J11", "Implement upstream-first ambiguity remediation and mandatory fresh re-audit", "E08", 3, "P0", "guidance",
    "Resolve findings at their true source, cascade corrections, and prove the repaired material independently.", [
        "Classify each finding by decision authority and source layer before proposing remediation.",
        "Present consequential client decisions clearly while resolving implementation detail through the configured firm.",
        "Repair Vision before Architecture, Architecture before interactions/contracts, and specifications before tasks/code.",
        "Invalidate/recompute downstream artifacts, plans, context, tasks, and active assumptions through graph impact.",
        "Run a fresh audit from source after remediation; do not merely check off the old finding list."
    ], deps=["J10", "J12"])
add("J12", "Implement decision propagation and feature evolution through the specification and implementation graph", "E08", 3, "P0", "guidance",
    "Handle corrections to existing intent differently from genuine new scope and cascade both safely.", [
        "Classify correction/propagation, new feature, new requirement, constraint, misunderstanding, or isolated implementation detail.",
        "Find the correct graph entry point and traverse affected downstream decisions, requirements, artifacts, contracts, tasks, code, tests, PRs, and releases.",
        "Present explicit contradictions, implicit assumptions, active-work risk, and migration/rollback consequences.",
        "Apply approved changes transactionally by layer with consistency checks and evolution/propagation artifacts.",
        "Never modify code alone when upstream product truth must also change."
    ], deps=["J08", "G11", "I07"])
add("J13", "Implement Autonomous, Collaborative, and Hands-on Deep Guidance engagement tiers", "E08", 4, "P1", "guidance",
    "Vary client interruption frequency without reducing discovery or specification depth.", [
        "Define which structural, product, architecture, risk, cost, and implementation decisions may auto-resolve at each tier.",
        "Require identical persisted reasoning/decision/coverage artifacts for auto-confirmed and interactive gates.",
        "Support changing tier mid-engagement without losing pending questions or silently approving blocked decisions.",
        "Group related decisions to avoid exhausting clients while preserving individual provenance.",
        "Expose pause/resume and next required gate across desktop and mobile."
    ], deps=["J01", "J08", "D09"])
add("J14", "Render the Specification Graph as versioned human artifacts and end-to-end requirements traceability", "E08", 4, "P1", "guidance",
    "Give clients and engineers readable vision, product, UX, architecture, contract, delivery, and implementation artifacts backed by structured truth.", [
        "Render executive and detailed views without duplicating contradictory standalone source documents.",
        "Support stable section/node references, diffs, comments, approvals, export to Markdown/JSON, and repository materialization.",
        "Trace idea → persona/workflow → requirement → decision → contract → task/issue → code → test → PR → deployment/release.",
        "Answer why-code-exists and whether-requirement-is-implemented using evidence paths.",
        "Flag human-readable render drift from structured source."
    ], deps=["J03", "J07", "J08", "F04", "G11"])

# ---------------------------------------------------------------------------
# E09 — Delivery and closure
# ---------------------------------------------------------------------------
add("K01", "Implement workload topology and non-functional constraint discovery", "E09", 3, "P0", "delivery",
    "Describe what the product must run and achieve before comparing vendors or copying a familiar stack.", [
        "Model request/response, long-running compute, jobs, queues, realtime/WebSockets, storage, data, search, graph/vector, inference, static/media, desktop/mobile/CLI, and platform builds.",
        "Capture load/burst/latency/availability, regions/residency, privacy/compliance, RPO/RTO, budget, team skill, lock-in tolerance, support, and growth assumptions.",
        "Separate hard constraints, preferences, estimates, unknowns, and testable hypotheses.",
        "Map every deployable component and data class to these constraints.",
        "Block provider selection when consequential workload assumptions remain ambiguous."
    ], deps=["J04", "J09"])
add("K02", "Implement current hosting, platform, CI/CD, distribution, and hybrid-architecture research", "E09", 3, "P0", "research",
    "Research the current market at planning time rather than relying on static skills or brand familiarity.", [
        "Compare credible cloud, edge, VPS/self-hosted, managed data, container, build, store, signing, and hybrid options applicable to the workload.",
        "Capture dated pricing, limits, regions, runtimes, cold starts, egress, quotas, networking, observability, SLA, support, and migration facts from primary sources.",
        "Treat hybrid topologies as first-class candidates and identify operational seams.",
        "Separate verified facts, estimates, negotiated/unknown pricing, and inference.",
        "Persist reusable provider evidence with freshness/refresh rules."
    ], deps=["K01", "A04"])
add("K03", "Implement delivery architecture option scoring, cost models, risk, lock-in, and recommendation artifacts", "E09", 3, "P0", "delivery",
    "Choose hosting and delivery topology from constraints and evidence with transparent tradeoffs.", [
        "Score fit, performance, availability, security, privacy, cost at representative loads, operational burden, developer experience, lock-in, failure modes, migration, and rollback.",
        "Model fixed, variable, egress, build, data, observability, support, and third-party costs with assumptions/sensitivity.",
        "Present at least one viable alternative and explain rejected options.",
        "Escalate genuine business tradeoffs to the client; keep implementation details with the firm.",
        "Lock the approved topology as an ADR and graph it before implementation planning."
    ], deps=["K02", "J08"])
add("K04", "Design CI/CD, preview, staging, promotion, migration, and rollback architecture", "E09", 3, "P0", "delivery",
    "Specify the complete path from issue/PR to verified artifacts and production, not just lint-test-build YAML.", [
        "Define branch/PR policy, affected-project detection, dependency order, caching, test partitioning, artifacts, SBOM/provenance, security scans, and required checks.",
        "Define preview, staging, production, canary/blue-green/staged rollout, manual gates, environment parity, and feature flags where applicable.",
        "Sequence schema/data migrations, backward compatibility, deploy, health, traffic switch, rollback, and forward-fix behavior.",
        "Define secret sources, signing credentials, least privilege, and protected-environment approvals.",
        "Provide failure-injection and rollback verification plans."
    ], deps=["K03", "M04"])
add("K05", "Design web, desktop, mobile, CLI, container, and package distribution channels", "E09", 3, "P0", "delivery",
    "Specify how every product surface reaches users and remains updated, supportable, and reversible.", [
        "Cover web/CDN/DNS/TLS, Linux packages, Windows installer/store/package managers, macOS signed/notarized artifacts, iOS/TestFlight/App Store, Android tracks/stores, CLI registries, GitHub Releases, and container images as applicable.",
        "Define signing, notarization, artifact identity, checksums, provenance, update feeds, release channels, staged rollout, rollback, and minimum supported versions.",
        "Separate build Host requirements from distribution service requirements.",
        "Define offline/air-gapped or self-hosted paths where product policy requires them.",
        "Map platform-specific compliance/review lead time into release planning."
    ], deps=["K03", "Q09", "Q10", "Q11"])
add("K06", "Build the production topology, environment, data, secrets, observability, backup, and support model", "E09", 3, "P0", "delivery",
    "Make runtime operations and failure recovery explicit before feature code assumes infrastructure behavior.", [
        "Graph environments, services, routes, dependencies, data stores/classes, queues, secrets, certificates, regions, ownership, dashboards, alerts, and runbooks.",
        "Define logs/metrics/traces, SLOs, alert thresholds, incident ownership, support diagnostics, retention, privacy, and cost controls.",
        "Define backup, restore, disaster recovery, key/certificate rotation, dependency outage, and vendor-exit procedures.",
        "Identify local versus managed operational responsibilities.",
        "Require a reachable staging topology and operational verification before dependent capability work is called production-ready."
    ], deps=["K03", "Q08"])
add("K07", "Implement architecture proof spikes, benchmarks, and evidence-to-decision workflows", "E09", 3, "P0", "delivery",
    "Resolve close or risky delivery choices with representative experiments instead of debate or toy demos.", [
        "Generate spikes from explicit hypotheses and uncertainty in delivery/architecture decisions.",
        "Use representative data, query patterns, concurrency, network, build, deployment, and failure scenarios.",
        "Record environment, versions, code, measurements, cost, limitations, and reproducibility.",
        "Compare baseline/candidates and update the ADR only from reviewed evidence.",
        "Clean up temporary infrastructure and secrets after the decision."
    ], deps=["A04", "K02"])
add("K08", "Implement multidimensional Capability Completion Contracts", "E09", 3, "P0", "closure",
    "Define what finished means independently from the number of planned implementation slices or tests.", [
        "Model applicable functional paths, states, data, migration, security, UX/responsive/accessibility, integration, failure/recovery, operations, quality, documentation, and delivery dimensions.",
        "Require REQUIRED, NOT_APPLICABLE with evidence, or approved DEFERRED debt for every dimension.",
        "Derive obligations from specifications, graph impact, product standards, delivery topology, and Role lenses.",
        "Version the contract and add newly discovered obligations without erasing original scope.",
        "Make closure status visible in tasks, PRs, Lead updates, and production readiness."
    ], deps=["J14", "K06", "D07"])
add("K09", "Implement Specification Graph ↔ Implementation Graph completeness and drift audits", "E09", 5, "P0", "closure",
    "Find specified-but-missing, implemented-but-unspecified, contradictory, partial, stale, and unverified behavior after code exists.", [
        "Map requirement/contract/state/error/access/edge-case/delivery obligations to code/tests/config/deploy evidence.",
        "Detect implicit decisions introduced by implementation and route them to formalization or correction.",
        "Compare actual blast radius and changed graph to planned tasks and Capability Contract.",
        "Generate remediation tasks rather than lowering the specification to match underdevelopment.",
        "Run again after remediation from fresh graph/spec sources."
    ], deps=["K08", "G11", "I12"])
add("K10", "Implement independent code, security, UX, delivery, and completeness review gates", "E09", 5, "P0", "closure",
    "Separate ‘is this code good?’ from ‘is this the whole production capability?’ and require independent evidence.", [
        "Assign reviewers through configured Roles with separation-of-duty policy and no self-approval where prohibited.",
        "Review code quality, correctness, security/privacy, architecture, UX/accessibility, operational behavior, delivery compatibility, tests, and Capability Closure.",
        "Ask reviewers to assume the implementation team underbuilt the feature and actively find missing work.",
        "Track findings to exact graph/spec/diff evidence and require disposition/retest.",
        "Invalidate approval when relevant code, spec, dependencies, or delivery topology changes."
    ], deps=["K09", "M07", "I10"])
add("K11", "Implement production-readiness, deploy verification, rollback, and recovery gates", "E09", 6, "P0", "closure",
    "Allow ‘done’ only when the actual capability is safely deliverable and operable in its target environments.", [
        "Require Capability and Impact Closure, reviews, local/CI checks, build artifacts, migration rehearsal, staging verification, observability, backup/rollback, security, and support evidence according to applicability.",
        "Verify deployed version/config/health and representative end-to-end behavior, not HTTP 200 alone.",
        "Exercise rollback or documented recovery in a safe environment for high-risk changes.",
        "Produce a signed/immutable readiness artifact linked to commit, artifacts, environments, checks, and approvers.",
        "Fail closed on stale, unknown, or mismatched evidence."
    ], deps=["K10", "M09", "K04", "K05"])
add("K12", "Prevent underdeveloped slices by expanding plans from Capability and Impact Closure", "E09", 4, "P0", "closure",
    "Make tasks/slices execution units that expand when reality reveals more work instead of definitions that cap product completeness.", [
        "Generate initial tasks from Capability Contract, specification graph, affected domains/surfaces, delivery obligations, and natural dependency seams.",
        "Do not merge independent flows merely to hit a slice count or split connected transactional work arbitrarily.",
        "Add tasks when graph indexing, implementation, review, runtime, or delivery reveals additional affected obligations.",
        "Block task completion when its acceptance passes but owned Capability obligations remain unresolved or unassigned.",
        "Measure coverage by traced obligations and evidence, not test-count ratios alone."
    ], deps=["K08", "I07", "D10"])

# ---------------------------------------------------------------------------
# E10 — Desktop workbench
# ---------------------------------------------------------------------------
add("L01", "Choose and scaffold the Linux-first desktop shell around the remote-capable client SDK", "E10", 4, "P0", "desktop",
    "Build a Linux reference desktop that is a client of symbioted rather than the owner of agent/task/process state.", [
        "Evaluate Tauri/Electron or credible alternatives against embedded browser/editor/terminal, process isolation, accessibility, packaging, auto-update, and cross-platform constraints.",
        "Create a thin desktop boundary around the shared client SDK and Host connection manager.",
        "Support local bundled Host, external local Host, and remote Fabric connection modes.",
        "Keep renderer privileges minimal and use typed commands/events for OS integration.",
        "Establish performance budgets and a Linux development/package smoke test."
    ], deps=["H03", "A04"], platforms=["linux"])
add("L02", "Implement the Project-scoped adaptive shell, workspace tabs, sidebars, and collapsible footer", "E10", 4, "P0", "desktop",
    "Provide an information-dense layout that preserves center workspace area and remembers state per Project.", [
        "Implement Project switcher and Agents, Editor, Preview, Artifacts, and Graph center tabs.",
        "Implement switchable/collapsible left navigation, Project Control right sidebar, and bottom terminal/problems/output/test panel.",
        "Allow collapse, resize, keyboard navigation, focus restore, and sane minimum/maximum constraints.",
        "Persist layout per Project/client while synchronizing only appropriate preferences.",
        "Support laptop through ultrawide layouts without forcing all panels visible."
    ], deps=["L01", "B02"])
add("L03", "Implement the floating Lead widget in voice, chat, hybrid, pinned, and docked states", "E10", 4, "P0", "desktop",
    "Keep the primary client/Lead relationship omnipresent without permanently consuming editor or Preview space.", [
        "Provide reactive idle/listening/transcribing/thinking/orchestrating/speaking/action-required states.",
        "Expand into movable/resizable chat with composer, transcript, attachments, task/artifact references, and microphone controls.",
        "Support voice-only orb, manual chat, full chat+voice hybrid, pin, dock, minimize, and screen-reader alternatives.",
        "Preserve focus and target context from selected DOM element, file, diff, graph node, task, or artifact.",
        "Render the selected Lead harness truthfully while keeping Symbiote events/structured activity distinct from chat prose."
    ], deps=["L02", "D02", "F01"])
add("L04", "Implement pluggable local/cloud speech-to-text and text-to-speech with coding vocabulary correction", "E10", 4, "P0", "voice",
    "Make dictation and spoken responses reliable for symbols, filenames, models, tools, and project terminology while preserving privacy choice.", [
        "Support push-to-talk, dictation preview/edit, hands-free/VAD conversation, interruption, and configurable hotkeys.",
        "Define STT/TTS provider adapters including local/offline models, OS-native options, and user-supplied/cloud endpoints.",
        "Bias/correct against Project graph symbols, paths, packages, Role names, harness/model names, MCPs, and recent conversation with visible reversible corrections.",
        "Read only conversational response by default, with Natural/Developer/Literal speech modes for code and structured UI.",
        "Expose latency, privacy/data path, model/resource needs, language, and failure fallback."
    ], deps=["L03", "G14", "N08"])
add("L05", "Build the responsive Agent and Fabric Floor", "E10", 4, "P0", "desktop",
    "Show the AI firm working as Role-centric responsive cards rather than terminal tiles.", [
        "Display Role, assignment, status, task, progress evidence, changed files, diagnostics, tests, review, harness/model metadata, Host, and blockers.",
        "Represent waiting/blocked reasons and dependency readiness, not just idle.",
        "Support responsive rows/grid, grouping by Objective/Host/status, compact/expanded cards, and drill-down to session/diff/Preview.",
        "Stream updates efficiently without card reordering that destroys spatial orientation.",
        "Provide keyboard and screen-reader parity with pointer interactions."
    ], deps=["L02", "D10", "H06", "F02"])
add("L06", "Build switchable Sessions, Files, Search, and Source Control left navigation", "E10", 4, "P1", "desktop",
    "Give fast navigation without recreating an undifferentiated sidebar session history.", [
        "Group sessions under current/past Objectives and Roles with state/attention indicators.",
        "Expose Files, workspace search, and Source Control as mode tabs with context-sensitive defaults.",
        "Preserve independent expansion/selection/scroll state per Project and mode.",
        "Support reveal active file/task/session and global command-palette navigation.",
        "Virtualize large trees/lists and surface stale/offline roots explicitly."
    ], deps=["L02", "F01", "B03", "M12"])
add("L07", "Build the Project Control sidebar for current goal, global tasks, Team, health, closure, and activity", "E10", 4, "P0", "desktop",
    "Answer what the firm is accomplishing, what remains, who is working, and what needs client attention at a glance.", [
        "Show current Objective/Capability, evidence-based progress, task DAG summary, critical path, active/waiting Roles, Hosts, diagnostics, tests, PRs, Impact/Capability Closure, and approvals.",
        "Provide Overview and Activity modes plus filters and drill-down into center tabs.",
        "Separate new regressions from existing baselines and unknown/stale status from healthy.",
        "Collapse cleanly and support a narrow attention rail.",
        "Synchronize acknowledgement/approval state across clients."
    ], deps=["L02", "D10", "F07", "I12", "K08"])
add("L08", "Implement the multi-root Project file tree and workspace search", "E10", 4, "P0", "desktop",
    "Provide VS Code-quality file navigation with Symbiote ownership, graph, diagnostic, and Git intelligence.", [
        "Support create/rename/move/delete/multiselect/drag-drop/reveal/filter, compact folders, ignored/generated handling, and multiple roots.",
        "Show Git, diagnostics, human/agent editing, task ownership, read-only/offline, generated, and conflict-risk decorations.",
        "Provide exact, regex, symbol, path, semantic, and graph-assisted workspace search with previews.",
        "Respect Host/path permissions and do not perform file mutation through stale cached state.",
        "Handle very large repositories through virtualization and incremental updates."
    ], deps=["L06", "B03", "G04", "M03"])
add("L09", "Implement the Monaco editor, LSP navigation, diffs, diagnostics, and graph actions", "E10", 4, "P0", "desktop",
    "Let users inspect and safely edit source without leaving Symbiote while integrating the unique System Graph intelligence.", [
        "Support tabs/splits, syntax, folding, find/replace, autocomplete, hover, definition/references, rename, formatting, code actions, diagnostics, and save/conflict behavior.",
        "Provide side-by-side and inline Git/task/agent diffs with comments and staging where authorized.",
        "Add Show Dependency Graph, Callers, Blast Radius, Tests, History, Tasks, and Ask Lead actions on symbols/ranges.",
        "Warn on editing agent-owned files and emit human-change events to the owning task.",
        "Remain useful when an LSP is unavailable while clearly degrading semantic actions."
    ], deps=["L08", "G14", "I07", "Q04"])
add("L10", "Implement the terminal multiplexer and human/agent process views", "E10", 4, "P0", "desktop",
    "Provide multiple persistent tabs/splits without mixing opaque agent processes into a confusing single terminal list.", [
        "Create/name/rename/reorder/split/restart/kill terminals with shell/profile/cwd/environment and per-Project persistence policy.",
        "Separate user terminals, declared Project commands/dev servers, and inspectable agent/runtime processes.",
        "Support resize/backpressure/scrollback/search/copy/paste/links/ANSI, process exit, reconnect, and remote Host terminals.",
        "Enforce terminal permission scopes and visible Host/worktree identity.",
        "Recover or explicitly mark lost terminals after Host/client restart."
    ], deps=["L02", "H02", "Q02"])
add("L11", "Implement the Preview Browser as an isolated application design and testing surface", "E10", 4, "P0", "preview",
    "Preview the user's running web application inside Symbiote without conflating it with an agent's general web-research browser.", [
        "Launch/connect declared dev servers and navigate/reload/back/forward/open route with per-Project cookies/storage profiles.",
        "Isolate Preview web content from privileged desktop APIs and Host credentials.",
        "Support desktop/tablet/mobile/custom viewport, device scale, theme, authentication persona/role, and saved reproduction state.",
        "Distinguish user Preview controls from separately permissioned agent browser automation controls.",
        "Capture screenshots/state/artifacts linked to commit, route, viewport, and task."
    ], deps=["L01", "L10", "B04", "Q02"])
add("L12", "Implement DOM-to-source/component/graph mapping and staged direct visual edits", "E10", 5, "P0", "preview",
    "Let a client select a rendered element, understand its source, experiment visually, and apply a coherent code patch.", [
        "Inspect DOM, computed styles, layout, accessibility identity, framework component/source map, source range, and graph node where available.",
        "Support temporary text, color, typography, spacing, size, border, radius, and layout changes with immediate preview.",
        "Accumulate preview modifications separately from source and show exact before/after plus affected selectors/components.",
        "Use the configured Frontend Role or deterministic transform to create a coherent source patch only on Apply.",
        "Handle generated CSS, design tokens, utility classes, media queries, missing source maps, and remounting honestly."
    ], deps=["L11", "L09", "G04", "D05"])
add("L13", "Implement Figma-style Preview annotations and direct orchestrator feedback", "E10", 5, "P0", "preview",
    "Let users comment on exactly the element and state they mean and route that feedback into canonical work.", [
        "Anchor a pin/comment to DOM locator, component/source/graph node, screenshot crop, route, viewport, role/persona, theme, application state, commit, and task.",
        "Send structured feedback to the Lead with user wording plus exact target context.",
        "Group annotations into versioned Design Feedback artifacts and convert selected/all unresolved items into tasks.",
        "Track open/resolved/outdated and remap targets across DOM/source changes with ambiguity handling.",
        "Support desktop and mobile creation/review of annotations."
    ], deps=["L12", "F05", "D02"])
add("L14", "Implement Preview console, network, accessibility, performance, storage, and runtime diagnostics", "E10", 4, "P0", "preview",
    "Feed browser problems into the same continuous engineering loop that handles compiler, lint, test, and CI failures.", [
        "Capture console errors/warnings, unhandled exceptions, failed requests, status/timing, WebSocket issues, hydration/render errors, storage and service-worker state.",
        "Run accessibility and selected performance audits tied to route/viewport/persona/commit.",
        "Correlate stack/source maps, network endpoints, graph nodes, task owner, and newly introduced versus baseline errors.",
        "Route actionable diagnostics immediately to the responsible task/Role with deduplication.",
        "Allow privacy-safe capture/export and prevent secrets/tokens from appearing unredacted."
    ], deps=["L11", "Q04", "Q05", "G07"])
add("L15", "Build the Artifacts center view", "E10", 4, "P0", "desktop",
    "Provide a dedicated professional workspace for specifications, plans, audits, reviews, designs, comments, versions, and delivery evidence.", [
        "Browse/filter/group by stage, type, Objective, Capability, Task, status, Role, version, and attention state.",
        "Render structured and document artifacts, attachments, provenance, graph links, comments, approval, versions, and comparisons.",
        "Support editing through controlled new revisions, anchored comments, task derivation, export/materialization, and mobile-compatible links.",
        "Show dependency invalidation and stale/superseded state prominently.",
        "Virtualize large artifact sets and avoid loading entire artifacts into UI or AI context unnecessarily."
    ], deps=["L02", "F03", "F04", "F05", "F06"])
add("L16", "Build the interactive System Graph center view", "E10", 5, "P1", "desktop",
    "Expose architecture, dependencies, blast radius, dead code, policies, provenance, and requirement traceability visually.", [
        "Support saved query/result views, path tracing, typed filters, certainty, environment/commit/time, clustering, expansion, and source navigation.",
        "Open selected nodes in Editor, Artifacts, Tasks, Preview, Git, or Lead context.",
        "Render very large graphs through query-bounded subgraphs, summaries, and progressive expansion rather than hairballs.",
        "Allow result annotations and task/artifact creation with query evidence.",
        "Provide accessible tabular/path alternatives to visual graph interaction."
    ], deps=["L02", "G14", "I07", "I08"])
add("L17", "Implement command palette, keybindings, accessibility, layout persistence, themes, and terminology modes", "E10", 4, "P1", "desktop",
    "Make the dense workbench fast, inclusive, customizable, and branded without forcing private vocabulary.", [
        "Create a searchable command registry for navigation, Project/Host/agent/task/Git/Preview/artifact/graph/terminal actions with permission-aware availability.",
        "Support configurable conflict-detected keybindings, focus management, reduced motion, contrast, zoom, screen readers, and keyboard-only operation.",
        "Persist layouts/themes/preferences per intended scope and recover corrupt layout state.",
        "Render canonical concepts through Standard, Balanced, or Immersive terminology dictionaries across desktop/mobile/web while CLI/API remain canonical.",
        "Never rename conventional engineering primitives such as Task, Issue, PR, Worktree, Review, or Artifact into confusing lore."
    ], deps=["L02", "B01", "B07"])

# ---------------------------------------------------------------------------
# E11 — Git and GitHub
# ---------------------------------------------------------------------------
add("M01", "Define and implement the Git/repository/provider abstraction for multi-root Projects", "E11", 1, "P0", "git",
    "Provide canonical repository, remote, branch, commit, worktree, dirty-state, and provider identities across Hosts.", [
        "Support plain Git first with provider-specific extensions rather than embedding GitHub assumptions in core domain objects.",
        "Handle multiple repositories/remotes, detached HEAD, unborn branch, submodules, LFS, sparse checkout, worktree metadata, and protected paths.",
        "Normalize status/diff/history with exact commit provenance and bounded performance.",
        "Define credential delegation without storing remote secrets in portable Project manifests.",
        "Provide safe transactions and recovery for interrupted Git operations."
    ], deps=["B01", "H01"])
add("M02", "Implement automatic task-owned branches and Git worktrees", "E11", 2, "P0", "git",
    "Isolate mutating tasks automatically and bind work to canonical Task identity rather than a particular model or session.", [
        "Create deterministic collision-resistant branch/worktree names from Project/task/provider policy.",
        "Use no worktree for verified read-only tasks, optional policy for sole mutation, and mandatory isolation for parallel mutation.",
        "Record base commit, branch, path, Host, owner task, active dispatch, lifecycle, and cleanup eligibility.",
        "Validate repository state and dependencies before dispatch; never share one writable worktree between unrelated tasks silently.",
        "Support reassignment/fallback resuming the same task worktree."
    ], deps=["M01", "D07"])
add("M03", "Implement file/symbol ownership, human-edit coordination, and collision prevention", "E11", 4, "P0", "git",
    "Coordinate agents and users before concurrent edits destroy work or assumptions.", [
        "Track predicted and actual file/symbol/contract ownership by task and worktree.",
        "Warn when a human edits an agent-owned target and offer read-only, edit anyway, or request release.",
        "Emit human-change events/context deltas so workers cannot overwrite unseen client changes.",
        "Use graph overlap to detect semantic collision even across different files.",
        "Define conflict resolution, ownership transfer, and stale-editor save behavior."
    ], deps=["M02", "I11", "H03"])
add("M04", "Implement the GitHub connection and opinionated issue-first delivery workflow", "E11", 2, "P0", "github",
    "For GitHub Projects, make every material change traceable from canonical Symbiote work to GitHub issue before implementation.", [
        "Connect repositories with least required permissions and explicit organization/repository identity.",
        "Map Client Requests/Objectives/Capabilities/tasks to GitHub issues at the correct granularity; never create issues for every agent-local todo.",
        "Create/update/link issues idempotently and ingest relevant external comments, labels, state, assignees, and relationships.",
        "Preserve Symbiote as live orchestration truth and GitHub as durable collaboration/delivery truth with documented conflict rules.",
        "Allow plain-Git/no-provider mode without disabling canonical tasks."
    ], deps=["M01", "D07", "H04"])
add("M05", "Implement commit/checkpoint policy, branch naming, provenance, and task handoffs", "E11", 2, "P0", "git",
    "Produce coherent recoverable Git history from autonomous workers.", [
        "Define checkpoint versus final commits, author/co-author/provenance policy, message format, signing policy, and forbidden generated noise.",
        "Require diagnostics/tests appropriate to checkpoint claims and prevent commits containing secrets or unrelated work.",
        "Record commit-to-task/dispatch/artifact/decision links in the System Graph.",
        "Support handoff/salvage through clean commit, patch, or explicit dirty-state artifact.",
        "Respect Project squash/rebase/merge policy while preserving issue/task ancestry in metadata."
    ], deps=["M02", "F07"])
add("M06", "Implement draft pull-request creation, updates, evidence, and requirement traceability", "E11", 4, "P0", "github",
    "Open and maintain a reviewable PR as task work matures rather than dumping an unexplained final diff.", [
        "Create draft PRs with issue links, objective/capability, scope, implementation summary, decisions, tests, Impact/Capability Closure, risks, screenshots/artifacts, rollout, and rollback.",
        "Update managed sections idempotently while preserving human-authored discussion.",
        "Link commits/files/graph impact/spec requirements and flag unplanned diff content.",
        "Handle multi-repo tasks with linked PRs and explicit integration order.",
        "Never claim checks/review/closure that are stale against current head."
    ], deps=["M04", "M05", "G11", "I07"])
add("M07", "Implement independent PR review Roles, comments, findings, approvals, and remediation loops", "E11", 5, "P0", "github",
    "Route review to configured independent specialists and close findings with evidence before merge.", [
        "Assign code, security, architecture, UX, delivery, and completeness reviewers according to change/Capability policy.",
        "Read full current diff plus precise graph/spec/context, create inline/top-level findings with severity and evidence, and avoid duplicate comment spam.",
        "Map findings to remediation tasks/dispatches and verify fixes on a new head.",
        "Track human and AI approvals distinctly and enforce separation of duty.",
        "Dismiss/invalidate stale approval after relevant head or requirement changes."
    ], deps=["M06", "K10", "D03"])
add("M08", "Integrate GitHub Actions, checks, logs, artifacts, annotations, and automatic repair routing", "E11", 4, "P0", "github",
    "Treat CI as a live diagnostic source that routes failures directly to responsible work.", [
        "Ingest workflow/check runs, jobs, steps, annotations, logs, artifacts, retries, cancellation, and head SHA.",
        "Correlate failures to task/PR/files/graph nodes/Role and distinguish flaky/infrastructure/product failures.",
        "Route new failures immediately to owner or configured CI/QA Role with relevant log slices rather than full unbounded logs.",
        "Support rerun policy, required checks, environment approvals, and stale-run detection.",
        "Expose CI state in Project Control, Source Control, PR, mobile, and readiness artifacts."
    ], deps=["M06", "Q04", "D09"])
add("M09", "Implement graph-aware integration order, rebase/update, synthesis, merge, and issue closure", "E11", 6, "P0", "github",
    "Merge dependent PRs in a verified order and re-run affected closure after each integration.", [
        "Derive merge order from task DAG, contracts, graph dependencies, shared nodes, migrations, and delivery sequencing.",
        "Update/rebase dependent branches, recompute graph/diff impact, rerun selected checks, and invalidate stale approvals.",
        "Require policy gates for review, checks, Impact/Capability Closure, readiness, and client approval before merge.",
        "Support manual, auto-after-gates, merge queue, squash/rebase/merge policies, and rollback linkage.",
        "Close linked canonical/GitHub work only after verified merge and post-merge state."
    ], deps=["M07", "M08", "I12", "K10"])
add("M10", "Implement worktree/branch/PR cleanup, salvage, and interrupted-operation recovery", "E11", 5, "P1", "git",
    "Remove temporary execution state safely without losing unpushed or unreviewed work.", [
        "Classify merged, superseded, cancelled, failed, orphaned, dirty, unpushed, externally modified, and still-referenced worktrees.",
        "Require salvage/export or explicit destructive approval before deleting unique work.",
        "Prune local/remote branches and worktrees according to retention policy after verifying GitHub/task references.",
        "Repair stale Git worktree metadata and interrupted rebase/merge/cherry-pick states.",
        "Audit every destructive cleanup action."
    ], deps=["M02", "M09", "Q06"])
add("M11", "Implement provider-neutral plain-Git delivery and future forge extension points", "E11", 5, "P1", "git",
    "Keep GitHub deeply integrated without making it a requirement for local/private or future GitLab/Forgejo users.", [
        "Provide branch/worktree/commit/review/merge workflow using Symbiote tasks/artifacts when no forge is connected.",
        "Define forge adapter contracts for issues, PR/MR, comments, checks, artifacts, permissions, and webhooks/events.",
        "Preserve canonical IDs and migrate/link work when a forge is connected later.",
        "Document unsupported provider operations clearly rather than approximating them with Git alone.",
        "Create conformance fixtures for a second forge adapter without committing to its full implementation in core launch."
    ], deps=["M01", "M04", "F03"])
add("M12", "Build the Source Control, worktree, PR, and Actions desktop surface", "E11", 4, "P1", "desktop",
    "Expose repository state and delivery evidence natively without forcing users into GitHub's website for routine work.", [
        "Show repositories/branches/worktrees, changes/staging, commits, issues, PRs, reviewers, checks/Actions, artifacts, conflicts, and merge readiness.",
        "Navigate from files/diffs/findings/check failures to tasks, sessions, graph nodes, artifacts, and Preview.",
        "Provide safe commit, push, update/rebase, review, rerun, approve, and merge controls according to permission/policy.",
        "Surface Host/worktree identity and stale/current-head status prominently.",
        "Support provider-neutral degradation when GitHub is not connected."
    ], deps=["M03", "M06", "M08", "L02"])

# ---------------------------------------------------------------------------
# E12 — MCP, skills, secrets, extensions
# ---------------------------------------------------------------------------
add("N01", "Define the Agent Environment desired-state schema", "E12", 2, "P0", "environment",
    "Describe which native resources each Project/Role/Harness Instance should have without pretending Symbiote itself is the pair coder.", [
        "Model MCP servers, skills, instructions, rules, commands, prompts, hooks, plugins/extensions, packages, permissions, environment, secrets references, scopes, versions, and source trust.",
        "Allow Project defaults, Role requirements, Harness Instance overrides, Host feasibility, and user-global resources with explicit precedence.",
        "Separate required, optional, disabled, inherited, unsupported, and conflict states.",
        "Reference secrets symbolically and prohibit secret values in portable manifests.",
        "Version the schema and preserve provider-specific extension fields."
    ], deps=["B04", "C04", "D04"])
add("N02", "Implement managed-field ownership, three-way merge, drift, backup, and runtime-version migrations", "E12", 2, "P0", "environment",
    "Reconcile native agent folders safely even when users and harnesses edit them outside Symbiote.", [
        "Track desired, last-applied, and observed native state plus exact managed fields/files.",
        "Perform semantic merges for JSON/TOML/YAML and adapter-defined safe strategies for markdown, executable config, directories, and generated files.",
        "Preserve unknown fields/comments where format tooling permits and never replace entire user config as the default.",
        "Preview diff, conflicts, reload/restart needs, secret effects, and rollback before apply.",
        "Detect upstream config migration and require adapter-versioned transforms with tests."
    ], deps=["N01", "C04"])
add("N03", "Implement Project environment detection, scaffold selection, missing-harness installation, and trust onboarding", "E12", 2, "P0", "environment",
    "Make onboarding easy while never executing untrusted project-local resources silently.", [
        "Detect existing .agents, .claude, .codex, .pi, .opencode, .gemini, .cursor, .kiro, .qwen, .kimi, .factory, and other registered runtime resources.",
        "Show which harnesses are installed/authenticated/compatible and let users choose what to scaffold or install.",
        "Import existing resources into an ownership/conflict plan before mutation.",
        "Respect each harness's project trust/approval model and fingerprint executable hooks/extensions/config.",
        "Verify generated environment by probing the actual harness, not only checking files."
    ], deps=["N02", "C03", "C05"])
add("N04", "Build the curated MCP catalog, manifest schema, search, trust, and compatibility registry", "E12", 3, "P0", "mcp",
    "Let users find credible MCP servers and know exactly what they install, where, and with which privileges.", [
        "Store publisher/source/license/version/transports/install command or URL, inputs/secrets, tool inventory, data access, network/filesystem/process permissions, supported harness projections, and verification status.",
        "Support search/filter by capability, service, harness, transport, trust, license, and local/remote execution.",
        "Pin versions/checksums where possible and warn on mutable/unverified install sources.",
        "Provide review/revocation/security-advisory workflow.",
        "Keep catalog metadata separate from executable packages and never auto-install from description alone."
    ], deps=["N01", "A05"])
add("N05", "Implement one-click MCP configuration across selected Roles and native harnesses", "E12", 3, "P0", "mcp",
    "Let a user select an MCP, enter required information once, choose Roles/harnesses/scopes, and receive verified native configurations.", [
        "Generate an input form from manifest schema with secret, non-secret, optional, transport, scope, and permission distinctions.",
        "Resolve Role assignments to actual Harness Instances/Hosts and preview each native config change.",
        "Store secrets through N08 and project only references/environment mechanisms supported by each harness.",
        "Apply atomically per harness, verify server discovery/tool inventory, and report partial failure with repair/rollback.",
        "Detect drift, server/tool changes, unavailable Host dependencies, and unsupported harness transport."
    ], deps=["N04", "N02", "N08", "D04"])
add("N06", "Build the Skills catalog and cross-harness Agent Skills projection model", "E12", 3, "P0", "skills",
    "Share portable on-demand expertise where standards permit while retaining native harness-specific skills and precedence.", [
        "Support Agent Skills-compatible packages, .agents/skills, harness-native skill paths, user/project scopes, aliases, precedence, activation/consent, scripts/assets, and allowed tools.",
        "Detect when Pi, Gemini, OpenCode, Claude, Codex, or another harness can consume shared paths versus requiring a native copy/link/config entry.",
        "Validate metadata, name collisions, path layout, security, executable content, and version/source trust.",
        "Use progressive disclosure and expose expected context cost.",
        "Never claim one skill is semantically portable when a harness cannot honor its tools or activation model."
    ], deps=["N01", "N02", "A05"])
add("N07", "Manage native commands, hooks, rules, prompts, workflows, plugins, and extensions per harness", "E12", 3, "P1", "environment",
    "Give users one inventory and assignment UI while preserving fundamentally different runtime resources.", [
        "Represent resource type, native format/location, scope, precedence, trigger, permissions, executable code, trust fingerprint, reload behavior, and compatible harness versions.",
        "Support per-Role desired state without converting Gemini TOML commands, Claude hooks, Pi extensions, Kiro steering, or other resources into fake universal equivalents.",
        "Preview native installation/removal and preserve unrelated user resources.",
        "Route hook/resource events into Symbiote only through explicit adapter contracts.",
        "Quarantine changed/untrusted executable resources until re-approved."
    ], deps=["N01", "N02", "Q02"])
add("N08", "Implement the encrypted secret broker, OS keychain integration, scoped leases, and safe materialization", "E12", 2, "P0", "security",
    "Keep credentials out of manifests and minimize which Role, harness, Host, task, process, and environment can access them.", [
        "Use OS keychains/secret services where available with an encrypted local fallback and explicit backup limitations.",
        "Model secret owner, project, Role, Harness Instance, Host, environment, capability, expiry, rotation, and production sensitivity scopes.",
        "Issue short-lived environment/file/stdin/header leases according to adapter/runtime support and clean up materialized files.",
        "Redact secrets in UI, logs, artifacts, diagnostics, context, terminal capture, crash reports, and exports.",
        "Provide access audit, revoke/rotate, missing-secret diagnosis, and compromised-device response."
    ], deps=["Q01", "H04", "B01"])
add("N09", "Implement Role-driven tool, skill, secret, and permission assignment with least privilege", "E12", 3, "P0", "environment",
    "Make an AI employee's abilities and access a property of its Role/task, not whatever happens to be globally installed.", [
        "Resolve Role requirements to native harness resources and Host-local availability before dispatch.",
        "Compute least-privilege filesystem, shell, network, Git, MCP tool, secret, and environment access per task.",
        "Require explicit elevation with reason, duration, and approval policy.",
        "Prevent a fallback harness/Host from inheriting access merely because the primary had it.",
        "Show an exact access manifest in dispatch and review evidence."
    ], deps=["N05", "N06", "N07", "N08", "D04"])
add("N10", "Implement reproducible Project environment restore and compatibility repair", "E12", 4, "P1", "environment",
    "Restore a cloned Project on a new Host/Fabric without pretending missing dependencies, auth, secrets, or OS capabilities do not matter.", [
        "Compare manifest desired state to detected Hosts/harnesses/versions/resources/tools/secrets/commands/containers and produce an ordered restore plan.",
        "Allow install, substitute, defer, bind existing, or block decisions with impact and portability warnings.",
        "Apply native projections transactionally and verify each harness/resource.",
        "Start graph/database/services only after compatibility gates pass.",
        "Produce a restore artifact and leave Project in an explicit READY/DEGRADED/BLOCKED state."
    ], deps=["N03", "N09", "B07", "H07"])
add("N11", "Implement the signed extension SDK, capability permissions, marketplace, and revocation model", "E12", 7, "P1", "extension",
    "Let the community add Harness Packs, analyzers, transports, context providers, panels, commands, and delivery integrations without unsafe core forks.", [
        "Define stable extension points, manifests, permissions, process/sandbox boundary, version compatibility, lifecycle, storage, UI contributions, and diagnostics.",
        "Require explicit capabilities for filesystem, network, processes, secrets, graph queries, UI, tasks, Git, and Host administration.",
        "Support local development, signed publication, publisher identity, review, updates, pinning, disable, revoke, and vulnerability notices.",
        "Prevent third-party extensions from unrestricted direct canonical-database mutation.",
        "Create conformance/sample extensions for one Harness Pack, graph analyzer, and UI panel."
    ], deps=["A05", "C01", "G03", "H02", "Q02"])

# ---------------------------------------------------------------------------
# E13 — Mobile and remote
# ---------------------------------------------------------------------------
add("P01", "Implement the shared desktop/web/mobile client state runtime", "E13", 5, "P0", "mobile",
    "Reuse one authenticated RPC, cache, event, command, and conflict implementation across clients.", [
        "Package Host/Fabric discovery, pairing/session auth, snapshots, subscriptions, event replay, offline cache, optimistic commands, errors, and telemetry behind platform-neutral APIs.",
        "Keep file system, secure storage, networking, notifications, background execution, and biometrics behind platform adapters.",
        "Define mobile bandwidth/battery/background constraints and bounded raw-stream behavior.",
        "Provide deterministic fake Host/Fabric integration tests.",
        "Prevent mobile-specific shortcuts from creating a second canonical state model."
    ], deps=["H03", "H04", "H07"])
add("P02", "Research and lock the cross-platform mobile architecture and component-sharing strategy", "E13", 5, "P0", "research",
    "Choose React Native/Expo, Flutter, native, or another credible approach from Symbiote's actual Preview, voice, terminal, graph, security, and background requirements.", [
        "Prototype authenticated streaming, long lists, diff/code view, WebView Preview, microphone/TTS, push/deep links, secure storage/biometrics, background reconnect, and iPad/tablet layouts.",
        "Compare native capability, desktop/web code sharing, performance, binary/update policy, store compliance, accessibility, testing, and team complexity.",
        "Document platform limitations and where native modules are required.",
        "Select through an ADR and reproducible spike evidence.",
        "Do not let desire for maximum UI sharing override security or platform-quality requirements."
    ], deps=["P01", "A04"])
add("P03", "Build the first-class iOS/iPadOS Symbiote client shell", "E13", 5, "P0", "mobile",
    "Provide a production-quality Apple mobile/tablet client connected to the same Host/Fabric control plane.", [
        "Implement onboarding, pairing, Fabric/Project switcher, adaptive phone/tablet navigation, secure storage, accessibility, background/foreground lifecycle, and connection health.",
        "Support local LAN and remote Symbiote Link endpoints without separate Project identities.",
        "Respect scoped permissions and explain unavailable desktop-only presentation—not authority—features.",
        "Provide crash/offline recovery and state restoration.",
        "Pass device/simulator, networking transition, and accessibility test suites."
    ], deps=["P02", "H11"], platforms=["ios"])
add("P04", "Build the first-class Android phone/tablet Symbiote client shell", "E13", 5, "P0", "mobile",
    "Provide feature-equivalent Android control with platform-native security, background, and lifecycle behavior.", [
        "Implement onboarding, pairing, Fabric/Project switcher, adaptive phone/tablet/foldable navigation, secure storage, accessibility, lifecycle, and connection health.",
        "Support LAN and remote endpoints across Wi-Fi/cellular/VPN transitions.",
        "Respect scoped permissions and clear degraded states.",
        "Provide crash/offline recovery and state restoration.",
        "Pass emulator/device, network transition, background, and accessibility tests."
    ], deps=["P02", "H11"], platforms=["android"])
add("P05", "Implement mobile Lead chat, dictation, TTS, attachments, and action-required conversation", "E13", 5, "P0", "mobile",
    "Let the client direct the AI firm naturally from anywhere.", [
        "Render the same Lead session, structured activity separation, questions, approvals, task/artifact references, attachments, and voice state as desktop.",
        "Support push-to-talk, editable transcript, hands-free mode where platform-appropriate, TTS interruption, Bluetooth/audio route changes, and privacy indicators.",
        "Use project-aware vocabulary correction while keeping correction review usable on small screens.",
        "Queue/send safely through disconnect and prevent duplicate client commands.",
        "Deep-link from notifications to the exact Lead question/decision/task context."
    ], deps=["P03", "P04", "L04", "D09"])
add("P06", "Implement mobile Objectives, Task Graph, Agent/Fabric Floor, approvals, and intervention", "E13", 5, "P0", "mobile",
    "Allow meaningful project control rather than a read-only status companion.", [
        "Show current goal, evidence progress, tasks/dependencies/blockers, Roles, harness metadata, Hosts/Pulse, diagnostics, closure, and activity.",
        "Support answer/approve/reject, pause, steer, reassign, cancel, retry, and manual routing override according to permissions.",
        "Provide concise phone views and richer tablet layouts without hiding critical risk/state.",
        "Require confirmation and biometric/elevated authorization for configured consequential actions.",
        "Synchronize interventions exactly once across controllers."
    ], deps=["P03", "P04", "D10", "H07"])
add("P07", "Implement mobile artifact, diff, PR, CI, review, and merge workflows", "E13", 5, "P1", "mobile",
    "Let clients and reviewers inspect and approve real engineering evidence away from the workstation.", [
        "Browse/render artifact versions, comments, approval, requirements, source links, screenshots, and delivery evidence.",
        "Provide performant file/diff navigation, review findings/comments, PR summary, checks, logs slices, artifacts, and current-head status.",
        "Allow review/approve/request-changes/rerun/merge only under Project/device scope and required confirmations.",
        "Never present truncated diff/log context as complete without a clear boundary and fetch path.",
        "Support offline draft comments and conflict-aware submission."
    ], deps=["P03", "P04", "F05", "M07", "M08", "M09"])
add("P08", "Implement mobile Preview, annotations, files, graph, and scoped terminal access", "E13", 6, "P1", "mobile",
    "Expose the most valuable inspection/intervention surfaces in mobile-appropriate forms.", [
        "Open authenticated Preview routes through reachable Host/tunnel services and create element/screenshot annotations with viewport/device state.",
        "Browse/search/read files and compact diffs with source/task/graph links.",
        "Show saved graph queries, blast-radius paths, unresolved impact, and tabular evidence.",
        "Provide terminal access only for explicitly authorized devices/Roles with clear Host/cwd/worktree identity and session takeover rules.",
        "Avoid unrestricted desktop mirroring as the primary mobile experience."
    ], deps=["P07", "L13", "L16", "L10", "H11"])
add("P09", "Implement mobile device pairing, biometrics, scoped permissions, revocation, and risk UX", "E13", 5, "P0", "security",
    "Treat a phone as a separately permissioned controller, not an all-powerful copy of the workstation.", [
        "Store device credentials in platform secure hardware/keychain where available and bind them to Host/Fabric identity.",
        "Expose requested/granted scopes, Projects, expiration, last use, and sensitive-action biometric requirements.",
        "Support lost-device revocation, key rotation, re-pairing, screenshot/privacy controls, and notification-content policy.",
        "Use short-lived WebSocket/session tickets and certificate/pinning strategy justified by the transport model.",
        "Test rooted/jailbroken warning policy without falsely claiming perfect device trust."
    ], deps=["H04", "P03", "P04", "Q01"])
add("P10", "Implement privacy-minimized push notifications and deep links", "E13", 5, "P0", "mobile",
    "Alert the client when the firm needs them without sending source code, secrets, or sensitive prompts through push infrastructure.", [
        "Support needs-input, approval, review-ready, CI failure, impact expansion, Host offline, objective complete, and security events.",
        "Send opaque event/resource identifiers plus user-configured minimal preview text; fetch details end-to-end from the Host after authentication.",
        "Provide per-Project/event quiet hours, urgency, batching, redaction, and notification-device controls.",
        "Deep-link to the exact object and mark acknowledgement consistently across clients.",
        "Handle token rotation, revoked devices, duplicate delivery, and offline Hosts safely."
    ], deps=["P09", "D09", "H11"])
add("P11", "Implement mobile offline cache, reconnect, event replay, and concurrent-controller conflict handling", "E13", 5, "P0", "mobile",
    "Keep mobile trustworthy through cellular transitions and stale state without issuing duplicate or contradictory actions.", [
        "Define which project/task/artifact/diff data can be cached and its encryption/retention/sensitivity policy.",
        "Resume from durable cursors, refresh invalid snapshots, and surface last-updated/stale state.",
        "Queue only safe commands offline and require reconfirmation for stale consequential actions.",
        "Resolve simultaneous desktop/mobile approvals, edits, cancels, and merges deterministically.",
        "Run network chaos tests across Wi-Fi, cellular, VPN/Tailscale, tunnel failover, and app suspension."
    ], deps=["P01", "P09", "H03", "H07"])
add("P12", "Build Symbiote Link transport setup and diagnostics across desktop and mobile", "E13", 5, "P0", "network",
    "Make LAN, Tailscale, managed tunnel, custom HTTPS/WSS, and SSH understandable through one connection experience.", [
        "Detect viable transports, show privacy/cost/reachability, and recommend without forcing a hosted account.",
        "Enable LAN pairing, existing tailnet endpoint, managed outbound tunnel with QR, custom endpoint/certificate, and SSH profile flows.",
        "Show active path, latency, failover, certificate/identity, Host reachability, and actionable diagnostics.",
        "Prevent tunnel setup from bypassing Host authentication or exposing Preview/dev ports unintentionally.",
        "Support self-hosted/zero-cloud configuration and exportable connection profiles without private keys."
    ], deps=["H11", "P03", "P04", "L01"])

# ---------------------------------------------------------------------------
# E14 — Reliability, security, release
# ---------------------------------------------------------------------------
add("Q01", "Produce and enforce the Symbiote threat model and security-boundary architecture", "E14", 1, "P0", "security",
    "Define trust boundaries for a system that can run arbitrary agents, tools, hooks, terminals, extensions, repositories, tunnels, and deployments.", [
        "Threat-model client, Host daemon, Fabric, RPC/transports, harnesses, project files, MCPs, skills/hooks/extensions, Preview content, terminals, GitHub, secrets, mobile, tunnels, updates, and supply chain.",
        "Identify assets, actors, entry points, privilege transitions, abuse cases, mitigations, residual risk, and logging/response.",
        "Define untrusted Project onboarding and executable-config fingerprint/consent requirements.",
        "Map security controls to testable requirements/issues and update the model when architecture changes.",
        "Require independent security review of high-risk boundaries before release."
    ], deps=["A01", "A05"])
add("Q02", "Implement sandbox profiles, command/tool approval, Project trust, and Host execution policy", "E14", 2, "P0", "security",
    "Constrain AI and third-party execution according to Role/task/Project/Host risk rather than a global YOLO switch.", [
        "Define read-only, repository-write, task-worktree, build/test, network-limited, privileged, and custom sandbox profiles.",
        "Mediate shell/PowerShell, filesystem, network, MCP tools, browser automation, hooks, extensions, containers, and Host administration.",
        "Support allow/deny/ask/rewrite policies with exact action previews and noninteractive behavior.",
        "Respect upstream harness trust/approval while applying stricter Symbiote policy where configured.",
        "Log decisions and prevent prompt content from granting itself new privileges."
    ], deps=["Q01", "N09", "C01"])
add("Q03", "Implement secret lifecycle hardening, rotation, revocation, leak detection, and incident response", "E14", 4, "P0", "security",
    "Protect credentials throughout long-running, distributed, multi-agent development.", [
        "Scan staged changes, logs, artifacts, prompts/context, terminal output, crash data, and generated config for likely secrets with false-positive controls.",
        "Block or require explicit emergency override before commit/upload/deploy of credible secrets.",
        "Support secret rotation/revocation workflows and identify affected Roles/Hosts/MCPs/deployments through graph links.",
        "Expire leases and clean materialized credentials after process/task completion.",
        "Create an incident artifact and auditable containment steps without echoing the secret."
    ], deps=["N08", "Q01", "M05"])
add("Q04", "Implement the continuous Diagnostics Bus and pre-existing baseline model", "E14", 3, "P0", "diagnostics",
    "Normalize LSP, compiler, lint, formatter, tests, dev server, browser, schema, security, CI, and platform diagnostics as live canonical evidence.", [
        "Define diagnostic identity, source, severity, location, graph node, commit/worktree, task/Role/Host, baseline/new/fixed/regressed/flaky/stale state, and raw evidence reference.",
        "Capture an explicit Project/worktree baseline so unrelated old errors are not blamed on a task.",
        "Deduplicate and update diagnostics across rapid edits while preserving history.",
        "Stream bounded updates to responsible workers and clients.",
        "Do not mark task readiness with unknown or stale diagnostic sources."
    ], deps=["H02", "G04", "D08"])
add("Q05", "Implement live repair loops for lint, types, tests, Preview runtime, security, and CI", "E14", 4, "P0", "diagnostics",
    "Fix machine-detectable regressions during implementation rather than in a separate cleanup iteration.", [
        "Run a fast loop for incremental LSP/type/lint/format/affected tests/Preview and checkpoint loop for broader build/integration/security checks.",
        "Route newly introduced failures immediately to the owning Role with precise context and bounded logs.",
        "Prevent infinite fix loops through attempt budgets, hypothesis/result records, and escalation.",
        "Re-run only evidence invalidated by an edit, then broaden based on risk/failure.",
        "Block Ready for Review while required new diagnostics remain unresolved."
    ], deps=["Q04", "I09", "D09"])
add("Q06", "Implement crash-safe Host, harness, task, session, worktree, and process recovery", "E14", 4, "P0", "reliability",
    "Resume or accurately classify work after client, daemon, harness, machine, or network failure without fabricating completion.", [
        "Persist intent and state transitions transactionally before external side effects where possible.",
        "On restart, reconcile processes, sessions, locks, worktrees, Git state, tasks, diagnostics, tunnels, and service placement.",
        "Classify resumable, retryable, salvageable, orphaned, failed, and requires-human-recovery states with evidence.",
        "Prevent duplicate dispatch/commit/PR/merge after replay.",
        "Exercise power-loss/kill/network-partition fault injection in CI."
    ], deps=["H01", "H03", "D08", "F01", "M02"])
add("Q07", "Implement Project/Fabric backup, export, import, restore, and disaster recovery", "E14", 6, "P1", "reliability",
    "Protect canonical specifications, tasks, artifacts, graph, settings, and evidence while respecting source and secret boundaries.", [
        "Define backup sets and consistency points for relational/event state, artifacts, manifests, graph databases, indexes, and optional sessions/logs.",
        "Exclude or separately protect secrets and machine-local caches.",
        "Support encrypted local/export destinations, retention, incremental backups, integrity verification, and versioned restore.",
        "Restore into a new Host/Fabric with path/identity/credential reconciliation and no silent remote mutation.",
        "Run scheduled recovery drills against declared RPO/RTO."
    ], deps=["B04", "B06", "F03", "G02", "H07", "N08"])
add("Q08", "Implement Host/Fabric/Project observability, health, support bundles, and opt-in telemetry", "E14", 4, "P1", "reliability",
    "Make local and distributed failures diagnosable without secretly uploading source, prompts, credentials, or private metadata.", [
        "Collect bounded structured logs, metrics, traces/correlations, health checks, resource attribution, queue depth, indexing lag, RPC/tunnel quality, and error rates.",
        "Expose local dashboards and redacted support bundles with preview before export.",
        "Make product telemetry explicitly opt-in with documented fields, retention, disable/delete, and no source/prompt content.",
        "Define alerts for Host loss, task stalls, graph lag, secret failure, storage pressure, tunnel outage, and repeated agent failure.",
        "Link operational incidents to tasks/artifacts/graph/service topology."
    ], deps=["H06", "F07", "Q01"])
add("Q09", "Package and distribute the Linux Host, CLI, and desktop reference implementation", "E14", 5, "P0", "release",
    "Ship a reliable Linux-first product across major developer distributions and headless systems.", [
        "Produce signed/checksummed portable archives plus justified deb/rpm/AppImage/Flatpak or repository channels based on architecture research.",
        "Support symbioted service install/uninstall/update, user versus system mode, XDG paths, desktop integration, CLI completion, logs, and clean removal.",
        "Test Fedora/RHEL-family and Ubuntu/Debian-family reference environments plus a documented compatibility matrix.",
        "Provide rollback/release channels and preserve Project data across update.",
        "Verify headless Strix Halo/Fedora Host installation separately from desktop installation."
    ], deps=["H01", "L01", "K05", "Q06"], platforms=["linux"])
add("Q10", "Implement Windows Host/desktop compatibility, PTY/process/service, paths, packaging, and validation", "E14", 7, "P0", "release",
    "Bring the same Host/client architecture to Windows without assuming POSIX process, permission, path, shell, or symlink behavior.", [
        "Implement Windows service/user startup, ConPTY/PowerShell/cmd/shell profiles, process trees/job objects, path/case/UNC/long paths, filesystem watchers, permissions, and worktrees.",
        "Validate required Harness Packs and clearly mark WSL versus native execution.",
        "Produce signed installer/package/update/uninstall and rollback behavior.",
        "Run golden workflow and crash/recovery/security tests on supported Windows versions.",
        "Document platform gaps rather than silently routing unsupported work."
    ], deps=["Q09", "C05", "M02", "Q06"], platforms=["windows"])
add("Q11", "Implement macOS Host/desktop compatibility, launchd, PTY, entitlements, packaging, and validation", "E14", 7, "P0", "release",
    "Bring the Host/client architecture to macOS and enable required Apple build/signing capabilities.", [
        "Implement launchd/user lifecycle, PTY/shell/process, filesystem notifications, paths/permissions, keychain, sandbox/entitlements, worktrees, and sleep/wake.",
        "Validate Harness Packs and Xcode/iOS simulator/build capabilities in Host advertisement.",
        "Produce signed/notarized universal or justified architecture-specific app/package/update/uninstall behavior.",
        "Run golden workflow and crash/recovery/security tests on supported macOS versions.",
        "Keep Apple signing credentials scoped to eligible release Roles/Hosts."
    ], deps=["Q09", "C05", "M02", "Q06"], platforms=["macos"])
add("Q12", "Implement signed artifacts, notarization, installers, update feeds, channels, and supply-chain provenance", "E14", 7, "P0", "release",
    "Deliver verifiable releases and controlled updates for every supported binary/client surface.", [
        "Generate checksums, signatures, SBOM, build provenance, release manifests, changelog, compatibility, and vulnerability attestations.",
        "Implement stable/beta/nightly or configured channels with staged rollout, rollback, minimum Host/client protocol, and skipped-version migrations.",
        "Automate macOS notarization, Windows signing, Linux package signing, mobile store artifacts, and CLI/container publication as applicable.",
        "Protect signing credentials through isolated release Roles/Hosts and protected CI environments.",
        "Verify installed artifact identity and update path in release tests."
    ], deps=["Q09", "Q10", "Q11", "K05", "K04"])
add("Q13", "Build the cross-platform golden end-to-end conformance and release test laboratory", "E14", 7, "P0", "testing",
    "Prove the complete AI-firm workflow across Hosts, clients, harnesses, GitHub, graph, guidance, Preview, mobile, CI, and delivery.", [
        "Define a representative sample Project and client request that exercises Deep Guidance, ambiguity remediation, Team routing, P0 harnesses, context, graph, worktrees, Preview feedback, diagnostics, PR/review/CI/merge, and closure.",
        "Run Linux as the full reference; run Windows/macOS Host/client and iOS/Android client conformance against shared protocol contracts.",
        "Inject harness failure, quota fallback, Host loss, mobile reconnect, graph lag, merge conflict, CI failure, and rollback.",
        "Collect immutable evidence by exact versions/commits/artifacts and compare release regressions.",
        "Block stable release if the golden path or required platform matrix is not green."
    ], deps=["Q12", "P11", "K11", "M09", "C09", "L14"])
add("Q14", "Establish performance, scale, resource, latency, and endurance budgets with continuous benchmarks", "E14", 4, "P0", "testing",
    "Keep a multi-Project, multi-Host, multi-agent graph-heavy ADE responsive and resource-aware over long unattended runs.", [
        "Set budgets for Host idle/load, client startup/switch, RPC/event latency, large lists, terminal throughput, graph ingest/query, retrieval, context assembly, Preview, memory/disk growth, mobile bandwidth/battery, and recovery.",
        "Create small/medium/huge monorepo, many-Project, many-Host, many-agent, long-session, and degraded-network fixtures.",
        "Attribute resource use by Project/service/task/harness and enforce configurable ceilings.",
        "Run soak tests for leaks, orphan processes, event/log growth, stale indexes, and reconnect storms.",
        "Publish benchmark regressions as release-blocking evidence at defined thresholds."
    ], deps=["H06", "G12", "I01", "L02", "Q08"])

# ---------------------------------------------------------------------------
# Validation and body rendering
# ---------------------------------------------------------------------------
ALL_KEYS = {"ROADMAP", *EPICS.keys(), *(i.key for i in ITEMS)}
if len(ALL_KEYS) != 1 + len(EPICS) + len(ITEMS):
    raise SystemExit("Duplicate roadmap keys")
for item in ITEMS:
    if item.epic not in EPICS:
        raise SystemExit(f"Unknown epic for {item.key}: {item.epic}")
    for dep in item.deps:
        if dep not in ALL_KEYS:
            raise SystemExit(f"Unknown dependency {dep} in {item.key}")
        dep_wave = EPICS[dep]["wave"] if dep in EPICS else next((x.wave for x in ITEMS if x.key == dep), 0)
        if dep_wave > item.wave:
            raise SystemExit(f"Wave inversion {item.key} wave {item.wave} depends on {dep} wave {dep_wave}")

# cycle validation among atomic items
item_map = {i.key: i for i in ITEMS}
visiting, visited = set(), set()
def dfs(k: str):
    if k in visited or k not in item_map:
        return
    if k in visiting:
        raise SystemExit(f"Dependency cycle at {k}")
    visiting.add(k)
    for d in item_map[k].deps:
        dfs(d)
    visiting.remove(k)
    visited.add(k)
for k in item_map:
    dfs(k)

KIND_CONSTRAINTS = {
    "harness-pack": [
        "Upstream behavior is temporally unstable: record the exact tested version/commit and retrieval date; do not implement from memory.",
        "A lower integration tier must degrade explicitly rather than presenting terminal text parsing as structured state.",
    ],
    "graph": [
        "Every graph fact must retain analyzer/source, commit or validity scope, confidence, and exact provenance where available.",
        "Absence of an edge is not proof of absence when analyzer coverage is incomplete; surface uncertainty.",
    ],
    "context": [
        "Never fill a context window merely because capacity exists; reserve output/headroom and send only justified evidence.",
        "Secrets and unauthorized graph/source nodes must be filtered before ranking or packaging.",
    ],
    "guidance": [
        "The selected Lead performs reasoning, but the native Guidance engine owns stage state, locks, coverage, and gates.",
        "Depth may not be reduced merely because client involvement is reduced.",
    ],
    "desktop": [
        "The desktop UI is a client of the Host protocol; canonical task/session/process state must not live only in renderer memory.",
        "Pointer, keyboard, screen reader, narrow laptop, and ultrawide behavior must be specified and tested.",
    ],
    "mobile": [
        "Mobile is a first-class controller with scoped authority, not an insecure desktop mirror or read-only notification app.",
        "All sensitive cached data and device credentials require platform-appropriate protection and revocation.",
    ],
    "security": [
        "Threat model the change and apply least privilege; prompts or project files cannot grant themselves authority.",
        "Security claims require adversarial tests and evidence, not configuration presence alone.",
    ],
    "closure": [
        "Passing originally planned tests is insufficient when specification, graph impact, delivery, or Capability obligations remain open.",
        "Every N/A, suppression, or deferral requires a scoped rationale and approving authority.",
    ],
}

GENERIC_DONE = [
    "Unit and integration tests cover normal, boundary, failure, interruption, and recovery behavior appropriate to this issue.",
    "Public schemas/APIs/configuration and user-visible behavior are documented; migration and rollback are included where state changes.",
    "The implementation emits observable evidence and actionable errors instead of swallowing unknown or degraded states.",
    "Security, privacy, accessibility, performance, and cross-platform applicability are explicitly reviewed and marked required, not applicable with rationale, or separately tracked.",
    "Work is delivered through the repository's issue → task-owned worktree/branch → PR → independent review → current-head checks workflow once that infrastructure exists.",
]

def link_for(key: str, numbers: dict[str, int]) -> str:
    n = numbers.get(key)
    return f"#{n}" if n else f"`{key}`"


def render_item(item: Item, numbers: dict[str, int]) -> str:
    epic_link = link_for(item.epic or "", numbers)
    constraints = KIND_CONSTRAINTS.get(item.kind, [])
    deps = "\n".join(f"- {link_for(d, numbers)}" for d in item.deps) or "- None beyond the epic's governing architecture."
    scope = "\n".join(f"- {x}" for x in item.scope)
    notes = "\n".join(f"- {x}" for x in [*constraints, *item.notes]) or "- Follow the product constitution and applicable Project/Role/Host policies."
    acceptance = "\n".join(f"- [ ] {x}" for x in [*item.scope, *GENERIC_DONE])
    return f"""<!-- symbiote-plan-key: {item.key} -->
# Outcome

{item.outcome}

## Epic and delivery position

- Epic: {epic_link}
- Roadmap key: `{item.key}`
- Wave: **{item.wave}**
- Priority: **{item.priority}**
- Capability class: **{item.kind}**

## Required scope

{scope}

## Dependencies

{deps}

## Non-negotiable implementation constraints

{notes}

## Acceptance criteria

{acceptance}

## Required verification evidence

- [ ] Link the exact implementation commit(s), PR, tests/checks, and current-head verification results.
- [ ] Attach or link any ADR, compatibility dossier, schema, benchmark, threat model, migration, screenshot, trace, or recovery evidence required by the scope.
- [ ] Demonstrate that no material task, graph impact, diagnostic, review finding, or Capability obligation owned by this issue remains unaccounted for.
- [ ] Record newly discovered scope as linked canonical work; do not silently omit it or weaken acceptance criteria to close this issue.
"""


def render_epic(key: str, numbers: dict[str, int]) -> str:
    e = EPICS[key]
    children = [i for i in ITEMS if i.epic == key]
    checklist = "\n".join(f"- [ ] {link_for(i.key, numbers)} — {i.title}" for i in children)
    waves = sorted({i.wave for i in children})
    return f"""<!-- symbiote-plan-key: {key} -->
# Purpose

{e['purpose']}

## Governing rule

This epic is a progress tracker. Child issues are the executable units. Do not close this epic until every child is complete or explicitly superseded/deferred through an approved roadmap decision, and the epic-level integration behavior has been independently verified.

## Delivery waves

{', '.join(str(w) for w in waves)}

## Child issues

{checklist}

## Epic completion gate

- [ ] All child issue acceptance criteria are satisfied against current heads.
- [ ] Cross-child integration, migration, recovery, security, performance, and documentation are verified.
- [ ] No unresolved dependency, ambiguity, Impact Closure item, or Capability Closure item is hidden by child issue completion.
- [ ] The master roadmap reflects the final status and any approved scope evolution.
"""


def render_master(numbers: dict[str, int]) -> str:
    epics_list = "\n".join(f"- [ ] {link_for(k, numbers)} — {EPICS[k]['title']}" for k in EPICS)
    by_wave = []
    for w in range(8):
        count = sum(1 for i in ITEMS if i.wave == w)
        names = [EPICS[k]['title'] for k in EPICS if EPICS[k]['wave'] == w]
        by_wave.append(f"- **Wave {w}:** {count} atomic issues" + (f"; epic starts: {', '.join(names)}" if names else ""))
    return f"""<!-- symbiote-plan-key: ROADMAP -->
# Symbiote v2 — master product and implementation program

Symbiote is an open-source AI software development firm. The human is the client: they provide ideas, outcomes, constraints, feedback, approvals, and business decisions. A user-selected Lead orchestrator converses and reasons, while Symbiote owns canonical Project state, Team Roles, task DAGs, context, graph evidence, worktrees, policy, review, and delivery.

This program contains **{len(EPICS)} epics and {len(ITEMS)} atomic issues**. It supersedes the pre-Fabric v1 planning bundle.

## Constitutional architecture

- **Dedicated Projects:** switching Project replaces the complete operating context, Team, Lead, tasks, artifacts, graph, settings, resources, Git, terminals, and layouts.
- **AI firm, not model picker:** users assign harness/model/account/Host profiles to stable Roles, then describe outcomes rather than naming models in normal prompts.
- **Selected Lead, Symbiote-owned truth:** no model transcript or provider-local todo list is authoritative.
- **Linux first, all major platforms required:** `symbioted`, CLI, and desktop are proven on Linux first; Windows and macOS Hosts/desktops and iOS/Android clients remain required.
- **Host and Fabric:** any machine may run a headless Host; trusted Hosts form a Fabric controlled by multiple desktop/mobile clients over one typed protocol.
- **Native harness environments:** `.claude/`, `.codex/`, `.pi/`, `.opencode/`, `.gemini/`, and other resources retain native semantics. Symbiote manages desired state through non-destructive adapters.
- **System Graph:** vectors/BM25/fuzzy search are entry points into an evidence-backed digital twin of product intent, code, contracts, data, runtime, infrastructure, history, tests, tasks, and releases.
- **Precise context:** context is task/Role/model/permission aware, graph-guided, token-budgeted, inspectable, progressive, and delta-based.
- **Deep Guidance:** competitor gaps, recursive breadth-before-depth ideation, cross-cuts, decisions, ambiguity audits/remediation, Delivery Architecture, and requirements traceability are native stateful capabilities.
- **Closure, not test theater:** Impact Closure and multidimensional Capability Closure prevent a passing but underdeveloped slice from being called complete.
- **Issue-first delivery:** for GitHub Projects, material work defaults to Issue → canonical Task → task-owned worktree/branch → PR → independent review → current-head checks → graph-aware merge.
- **Preview is a design surface:** the embedded Preview Browser is separate from an agent research browser and supports DOM/source selection, visual edits, and anchored client feedback.
- **Remote/mobile is foundational:** LAN, Tailscale, managed tunnel, custom HTTPS/WSS, and SSH are transport choices beneath the same authenticated Host protocol.
- **Terminology is optional presentation:** Standard, Balanced, and Immersive wording render the same canonical technical model.

## Epics

{epics_list}

## Dependency-ordered waves

{chr(10).join(by_wave)}

## Program completion gate

- [ ] Every epic is complete or explicitly removed by a superseding constitutional decision with full impact propagation.
- [ ] The golden end-to-end request-to-production workflow passes on Linux and required platform/client conformance matrices.
- [ ] Current competitor parity/gap audit confirms all table stakes are implemented or deliberately rejected with rationale.
- [ ] Security, recovery, context efficiency, graph quality, mobile remote control, delivery, and resource budgets meet release thresholds.
- [ ] No blocking ambiguity, unresolved high-risk impact, stale review/check, or incomplete Capability Closure remains.
"""


def api(method: str, path: str, payload=None, retries=6):
    url = f"{API}{path}"
    data = None if payload is None else json.dumps(payload).encode()
    headers = {
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "symbiote-roadmap-v2-bootstrap",
    }
    if TOKEN:
        headers["Authorization"] = f"Bearer {TOKEN}"
    for attempt in range(retries):
        req = urllib.request.Request(url, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                raw = resp.read()
                return json.loads(raw) if raw else None
        except urllib.error.HTTPError as exc:
            body = exc.read().decode("utf-8", "replace")
            if exc.code in (403, 429) and attempt + 1 < retries:
                reset = exc.headers.get("X-RateLimit-Reset")
                sleep = 2 ** attempt
                if reset and reset.isdigit():
                    sleep = max(sleep, int(reset) - int(time.time()) + 1)
                time.sleep(min(sleep, 90))
                continue
            raise RuntimeError(f"GitHub API {method} {path} failed {exc.code}: {body[:2000]}") from exc
        except OSError as exc:
            if attempt + 1 == retries:
                raise
            time.sleep(min(2 ** attempt, 30))


def paged(path: str):
    page = 1
    while True:
        sep = "&" if "?" in path else "?"
        rows = api("GET", f"{path}{sep}per_page=100&page={page}")
        if not rows:
            return
        yield from rows
        if len(rows) < 100:
            return
        page += 1


def ensure_labels():
    existing = {x["name"] for x in paged(f"/repos/{OWNER}/{NAME}/labels")}
    for name, (color, desc) in LABELS.items():
        if name in existing:
            continue
        if DRY_RUN:
            print(f"DRY label {name}")
        else:
            api("POST", f"/repos/{OWNER}/{NAME}/labels", {"name": name, "color": color, "description": desc})


def existing_issues():
    result = {}
    for issue in paged(f"/repos/{OWNER}/{NAME}/issues?state=all"):
        if "pull_request" in issue:
            continue
        m = MARKER_RE.search(issue.get("body") or "")
        if m:
            result[m.group(1)] = issue
    return result


def labels_for(key: str, epic_key: str | None, wave: int, priority: str, kind: str, platforms=()):
    labels = ["symbiote-roadmap", f"wave:{wave}", f"priority:{priority}"]
    if key == "ROADMAP": labels.append("type:master")
    elif key in EPICS: labels.append("type:epic")
    elif kind in ("research", "architecture"): labels.append("type:research")
    else: labels.append("type:feature")
    for p in platforms:
        labels.append(f"platform:{p}")
    return [x for x in labels if x in LABELS]


def upsert_issue(key: str, title: str, body: str, labels: list[str], existing: dict):
    current = existing.get(key)
    payload = {"title": title, "body": body, "labels": labels}
    if DRY_RUN:
        print(f"DRY {'update' if current else 'create'} {key}: {title}")
        return current or {"number": -1, "title": title, "body": body}
    if current:
        return api("PATCH", f"/repos/{OWNER}/{NAME}/issues/{current['number']}", payload)
    created = api("POST", f"/repos/{OWNER}/{NAME}/issues", payload)
    existing[key] = created
    return created


def main():
    if not TOKEN and not DRY_RUN:
        raise SystemExit("GITHUB_TOKEN is required")
    print(f"Validated plan: {len(EPICS)} epics, {len(ITEMS)} atomic issues")
    ensure_labels()
    existing = existing_issues()
    numbers = {k: v["number"] for k, v in existing.items()}

    # Pass 1: create/find all identities with bodies that may temporarily use plan keys.
    master = upsert_issue("ROADMAP", "[MASTER] Symbiote v2 — AI development firm product and implementation program",
                          render_master(numbers), labels_for("ROADMAP", None, 0, "P0", "master"), existing)
    numbers["ROADMAP"] = master["number"]
    for key, e in EPICS.items():
        row = upsert_issue(key, f"[EPIC {key[1:]}] {e['title']}", render_epic(key, numbers),
                           labels_for(key, None, e["wave"], "P0", "epic"), existing)
        numbers[key] = row["number"]
    for item in sorted(ITEMS, key=lambda x: (x.wave, x.epic or "", x.key)):
        row = upsert_issue(item.key, f"[{item.key}] {item.title}", render_item(item, numbers),
                           labels_for(item.key, item.epic, item.wave, item.priority, item.kind, item.platforms), existing)
        numbers[item.key] = row["number"]

    # Pass 2: replace every key reference with real issue numbers/checklists.
    existing = existing_issues()
    upsert_issue("ROADMAP", "[MASTER] Symbiote v2 — AI development firm product and implementation program",
                 render_master(numbers), labels_for("ROADMAP", None, 0, "P0", "master"), existing)
    for key, e in EPICS.items():
        upsert_issue(key, f"[EPIC {key[1:]}] {e['title']}", render_epic(key, numbers),
                     labels_for(key, None, e["wave"], "P0", "epic"), existing)
    for item in sorted(ITEMS, key=lambda x: (x.wave, x.epic or "", x.key)):
        upsert_issue(item.key, f"[{item.key}] {item.title}", render_item(item, numbers),
                     labels_for(item.key, item.epic, item.wave, item.priority, item.kind, item.platforms), existing)

    print(json.dumps({"repository": REPO, "master": numbers["ROADMAP"], "epics": len(EPICS), "atomic": len(ITEMS), "total": 1 + len(EPICS) + len(ITEMS)}, indent=2))

if __name__ == "__main__":
    main()
