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

No repository LICENSE, contribution policy or security disclosure file was present when this baseline was checked. This batch does not select a license, grant rights, declare dependency-license compatibility or invent a security contact. Core/official-pack/extension/catalog/hosted-service license boundaries, contributor/trademark/commercial policy, coordinated disclosure and enforceable dependency/provenance CI remain explicit #174 release requirements. A green build is not license, publisher or release-signing evidence.

This baseline consumes the #170 constitution. #174/#191 remain open; later architecture and integration changes must update this register and link exact adversarial evidence. #218 can use the consent boundary and threat register while its real containment work remains required.
