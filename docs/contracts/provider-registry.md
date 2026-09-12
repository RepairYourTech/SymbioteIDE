# Provider registry: durable model/provider identity

Canonical owner: [#464](https://github.com/RepairYourTech/SymbioteIDE/issues/464), depending on #36 and #184. `symbiote-store`'s provider tables plus protocol v1.9 implement the durable registry layer for the model/provider boundaries that the #184 SDK contracts (`symbiote-runtime-sdk::provider`) already define: `ProviderConnection`, `BillingEntitlement` and `ModelDescriptor` records become durable, replay-audited state instead of per-call parameters. This is not full #464 acceptance: dispatch/session flow, mixed-runtime schema round-trips and the AgentRuntimeAdapter persistence story remain pending.

## What is registered

- **Provider connections** — adapter identity, endpoint reference and authentication kind. Replacements supersede the stored record (new endpoint on the same identity); the journal keeps every revision.
- **Billing entitlements** — bound to a stored connection by foreign key, carrying kind, verification evidence and expiry. Expiry is *not* checked at registration time: it is checked when a dispatch is prepared, again when the dispatch starts, and again when it is activated — the execution boundary re-validates the contract's pinned profile before any worktree or transport side effect, so an entitlement that lapses while a dispatch is Running refuses the run.

The three records are consumed together by dispatch preparation: the runtime profile a workforce binding carries names a connection, an entitlement and a model, and preparation validates that triple through the SDK registration contract (`validate_registration`) — identity consistency, endpoint reference, descriptor bounds, unexpired entitlement and a supported authentication/billing mapping — recording a typed `provider` refusal when they do not agree. Absence of a record is a refusal, never an empty binding, so a registered connection alone no longer resolves a profile. The three boundaries — preparation, start and activation — resolve through that one implementation (`symbiote_store::provider::registration_for`), so a registration cannot be usable at one and unusable at another, and the reason a run is refused is the same typed reason preparation records — reported by name wherever the operator asks, never collapsed into a generic precondition.
- **Model descriptors** — SDK-owned content: context/output bounds travel with the record and are re-validated by the SDK contract on registration (`schema_version`, nonzero and consistent token bounds; capabilities are deserialized but not semantically validated).

Registration is attributed to a real Project chosen by the caller (the Project whose workforce intends to use the provider): journal rows are project-scoped, so provenance is explicit, while the records themselves are global identity usable by any Project after re-authorization at binding time. Attribution grants nothing: the record bodies are withheld from the attribution project's journal reads — non-owner readers learn only that a registration occurred, never the connection, entitlement or descriptor contents.

## Authority and boundaries

All registry writes require the local-owner bootstrap policy (`ReplaceProviderConnection`, `ReplaceBillingEntitlement`, `ReplaceModelDescriptor`), consistent with other Host-owned infrastructure; reads are owner-only as well, and journal events carrying registry payloads are filtered to the same authority. Restricted worker identities never touch this registry — least-privilege assignment (#269) will surface effective access separately. Provider records carry no secret values: `CredentialReference` remains a vault key reference, and secret materialization stays with the canonical secret broker (#217).

## Journal replay and integrity

Schema v9 adds `provider_connections`, `billing_entitlements` and `model_descriptors` with tamper-evident indexed columns (adapter/provider projections of the bodies, checked on every read and during replay). Journal replay validates each registration event's shape, attribution and request-byte equality, requires referenced connections to precede entitlements/models, and reconciles all three tables against the journal-derived maps in both directions — injected rows without provenance fail startup, exactly like bindings, teams, routes, dependencies and leases.

Protocol v1.9 adds the three `replace_*` operations (owner, with project attribution), three `get_*` reads (owner), the three journal payloads, and `ProviderRegistryWrite`/`ProviderRegistryRead` capabilities.

## Evidence and remaining acceptance

Covered by store tests: register/read/replay across restart, replacement supersession, replayed receipts, unknown-connection refusal, invalid descriptor refusal, tampered-row rejection, and dispatch preparation validating (or refusing) the profile's registered triple, plus the start and activation boundaries re-validating that triple (a registration that lapses after preparation refuses the start; one that lapses after start refuses the run before any worktree or transport side effect). Pending #464 acceptance, tracked in the issue: the live adapter/session flow the binding feeds (schema round-trips for external Lead/native worker and all-native staffing, runtime-kind preservation across changes, missing-CLI startup and local-endpoint-only configuration tests), and the AgentRuntimeAdapter/session persistence layer.

Applicability: security/privacy — owner-authority only, no secret values stored, cross-Project use re-authorized at binding time. Accessibility not applicable to a headless registry. Performance bounded by record counts, not benchmarked. Linux-first, matching the Host.
