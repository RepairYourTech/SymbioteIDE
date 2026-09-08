# Provider registry: durable model/provider identity

Canonical owner: [#464](https://github.com/RepairYourTech/SymbioteIDE/issues/464), depending on #36 and #184. `symbiote-store`'s provider tables plus protocol v1.9 implement the durable registry layer for the model/provider boundaries that the #184 SDK contracts (`symbiote-runtime-sdk::provider`) already define: `ProviderConnection`, `BillingEntitlement` and `ModelDescriptor` records become durable, replay-audited state instead of per-call parameters. This is not full #464 acceptance: dispatch/session flow, mixed-runtime schema round-trips and the AgentRuntimeAdapter persistence story remain pending.

## What is registered

- **Provider connections** — adapter identity, endpoint reference and authentication kind. Replacements supersede the stored record (new endpoint on the same identity); the journal keeps every revision.
- **Billing entitlements** — bound to a stored connection by foreign key, carrying kind, verification evidence and expiry. Validation against an expired entitlement happens at SDK binding time, not registration time.
- **Model descriptors** — SDK-owned content: context/output bounds and capabilities travel with the record and are re-validated by the SDK contract on registration (`schema_version`, nonzero bounds, capability sanity).

Registration is attributed to a real Project chosen by the caller (the Project whose workforce intends to use the provider): journal rows are project-scoped, so provenance is explicit, while the records themselves are global identity usable by any Project after re-authorization at binding time. Attribution does not scope visibility or grant authority.

## Authority and boundaries

All registry writes require the local-owner bootstrap policy (`ReplaceProviderConnection`, `ReplaceBillingEntitlement`, `ReplaceModelDescriptor`), consistent with other Host-owned infrastructure; reads are owner-only as well. Restricted worker identities never touch this registry — least-privilege assignment (#269) will surface effective access separately. Provider records carry no secret values: `CredentialReference` remains a vault key reference, and secret materialization stays with the canonical secret broker (#217).

## Journal replay and integrity

Schema v9 adds `provider_connections`, `billing_entitlements` and `model_descriptors` with tamper-evident indexed columns (adapter/provider projections of the bodies, checked on every read and during replay). Journal replay validates each registration event's shape, attribution and request-byte equality, requires referenced connections to precede entitlements/models, and reconciles all three tables against the journal-derived maps in both directions — injected rows without provenance fail startup, exactly like bindings, teams, routes, dependencies and leases.

Protocol v1.9 adds the three `replace_*` operations (owner, with project attribution), three `get_*` reads (owner), the three journal payloads, and `ProviderRegistryWrite`/`ProviderRegistryRead` capabilities.

## Evidence and remaining acceptance

Covered by store tests: register/read/replay across restart, replacement supersession, replayed receipts, unknown-connection refusal, invalid descriptor refusal, tampered-row rejection. Pending #464 acceptance, tracked in the issue: schema round-trips for external Lead/native worker and all-native staffing (needs dispatch flow), runtime-kind preservation across changes, missing-CLI startup and local-endpoint-only configuration tests, and the AgentRuntimeAdapter/session persistence layer.

Applicability: security/privacy — owner-authority only, no secret values stored, cross-Project use re-authorized at binding time. Accessibility not applicable to a headless registry. Performance bounded by record counts, not benchmarked. Linux-first, matching the Host.
