# Trust boundary baseline

Owners: #174 trust policy, #191 threat model, #218 preventive enforcement. This document records the current implementation and blocking residual risks; it is not release certification.

## Authority and ownership

Projects, source, prompts, logs and private configuration remain user-owned local data. The current Host has no telemetry upload or learning upload implementation. Any future telemetry, learning or maintainer export needs separate explicit consent, inspectable event schemas, retention/deletion controls and permission filtering before retrieval or aggregation. A resource appearing on disk, a skill instruction, a model response or a successful config projection supplies no authority.

The Host owns canonical state and decisions. Native agents and external harnesses are equally untrusted relative to that boundary. They must not access the canonical database directly or change evaluators, policy, billing, update channels or their own sandbox. The current service authenticates a local same-UID peer as the local owner; it does not distinguish a malicious same-UID worker from the user. **Worker activation remains disabled until OS isolation and a restricted worker protocol are proven.** Private filesystem permissions alone do not resolve this risk.

The current resource consent ledger records the local owner's decision over exact resource metadata and fingerprint. It is not publisher verification, executable approval by filename, or a sandbox. A future loader must observe the actual bytes/dependency closure, bind them to the consent, recheck current policy and revocation, prevent replacement between check and use, and install preventive controls before hooks/configuration execute. Consent for one Role/profile/Host never implies consent for another.

## Threat register

| Entry point / actor | Asset or transition | Current control / evidence | Remaining release gate |
|---|---|---|---|
| Compromised client or forged RPC fields | Host identity, journal, approvals | OS peer credentials; no wire principal/actor; local-owner-only consent mutation; protocol and daemon fixtures | Restricted worker identity, cross-platform authenticated transport and independent security review |
| Malicious repo/config/hook/skill/MCP | Files, commands, secrets and bootstrap | Exact fingerprint/context consent model; inactive projection; no auto-load | #218 filesystem/network/process/credential containment before any executable bootstrap; loader check/use binding |
| Native inference or external harness | Canonical DB and completion authority | No worker launch endpoint; task completion remains Host-owned; metadata-only protocol | Actual denied DB/RPC access under sandbox, tool mediation and verified termination of descendants |
| Prompt/tool-output injection or poisoned guidance | Permission or evaluator escalation | Resource content cannot issue Host approval; exact policy/root/grant comparisons and snapshot mutation fixtures | Adversarial native/external integration, provenance-aware Context Broker and poisoned MCP/docs tests |
| Stale/changed/revoked resource | Consent reuse | Durable exact resource consent, expiry, revocation journal, policy revision check and restart tests | Active-session revocation delivery, dependency fingerprints, race-free loading and publisher revocation |
| OAuth/account confusion | Native API billing and harness credentials | Separate runtime/provider contracts; no credential copying by projection | AUTH-01/AUTH-02 integration, client impersonation and entitlement tests |
| Untrusted Preview/browser content | Privileged Host bridge | Separate proof WebView; no production privileged Preview bridge | #38 actual bridge denial/containment tests; iframe placement is insufficient |
| Terminal or escaped child process | Host process tree and unrelated files | Bounded structured transport; explicit unknown descendant cleanup | #218 tested process-tree containment, cancellation and outstanding effect disposition |
| Poisoned learning, benchmarks or analytics | Security downgrades, verification and cross-Project data | No live learning/analytics worker; policy remains outside expertise resources | Permission-filtered retrieval/export, tamper evidence and cross-Project leakage tests |
| Compromised pack/dependency/update | Executable supply chain | Pinned Cargo dependencies and current-head CI; no catalog auto-install | Publisher identity/signatures, revocation, dependency/license policy, SBOM/provenance/signatures and update verification |
| Fabric/mobile/tunnels/device theft/GitHub delivery | Remote control, secrets and production effects | Not implemented or exposed by current Host | Separate authentication, step-up authority, revocation, split-brain, signing and deployment evidence |

Static error codes and bounded metadata avoid echoing raw payloads. Raw local config remains sensitive even when a projection preserves it. Logs and fingerprints are not an automatic permission to upload or aggregate data. Incident handling must preserve relevant canonical events, revoke authority and stop affected execution; active worker interruption is still unimplemented.

## Licensing and public-release obligations

No repository LICENSE, contribution policy or security disclosure file was present when this baseline was checked, and none is present now. No license has been selected, no rights are granted, and no security contact exists: selecting the core license, the contributor and trademark terms, the disclosure channel and the hosted-service terms remain explicit [#174](https://github.com/RepairYourTech/SymbioteIDE/issues/174) release requirements.

The boundaries those selections have to fit, and what is enforced here rather than promised, are recorded in [`planning/policy`](../../planning/policy/README.md) and read by `planning/policy/policy.py --check`. Five artifact classes (core, official pack, extension, catalog metadata, hosted service) state what each covers and what it does not, and a class that names a license the tree does not carry is refused — as is a `LICENSE` file no class accounts for. Every dependency this workspace links is recorded with the license it is used under and the class it lands in, read back from the manifests in both directions, and a license the distribution boundary cannot honour is refused; each generated artifact names the program that wrote it, the command that re-derives it and the case that holds it. Seventeen obligations the issue's criteria are made of each carry a status — built with the case that exercises it, deferred to the issue that owns it, or not applicable with a rationale — and a claim of `built` with no evidence in this tree is refused. A green build is still not license, publisher or release-signing evidence, and publisher identity, signing, revocation, release SBOM/provenance and telemetry retention remain [#431](https://github.com/RepairYourTech/SymbioteIDE/issues/431), [#434](https://github.com/RepairYourTech/SymbioteIDE/issues/434) and [#459](https://github.com/RepairYourTech/SymbioteIDE/issues/459).

This baseline consumes the #170 constitution. #174/#191 remain open; later architecture and integration changes must update this register and link exact adversarial evidence. #218 can use the consent boundary and threat register while its real containment work remains required.
