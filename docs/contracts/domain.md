# Domain contracts v1

Canonical owner: [#36](https://github.com/RepairYourTech/SymbioteIDE/issues/36), constrained by [#170](https://github.com/RepairYourTech/SymbioteIDE/issues/170) (closed) and the [product constitution](../architecture/product-constitution.md). This is a bounded executable foundation whose domain ontology is complete as a contract; what it is not — and does not claim to be — is a Host, persistence layer, authenticated API, runtime integration, Git implementation or persistent store.

`symbiote-domain` supplies Rust/Serde records, JSON Schema generation and deterministic pure transitions. Run `cargo test -p symbiote-domain` from the repository root; `cargo run -p symbiote-domain --example domain_schema` prints the machine-readable schema for `DomainEnvelope`, and `--example ontology_schema` prints the canonical vocabulary's published artifact (its `--write`/`--check` pair regenerates and diffs the committed copy). No network or installed external agent is needed for these contract tests. The native/external tests prove shared state semantics, not either execution integration.

## Identity, authority and compiled staffing

All internal identities are distinct Rust newtypes serialized as opaque strings. IDs accept 1–128 ASCII letters, digits, `_` or `-`; generation/uniqueness belongs to the future authority. Never derive them from external labels. External references are separately typed system/locator observations. A Role has no runtime identity: re-staffing changes its versioned Binding and creates a new Dispatch. The immutable dispatch snapshot includes the precise Role/operating contract, Binding/Workforce Protocol, Runtime Profile/model/authentication/billing references, eligible selected Host revision, Task contract/revision, context budget, tool/skill requirements, access and verified enforcement claims. Equivalent pinned compilation inputs produce equal snapshots.

Compilation rejects cross-Project or Role lineage, mismatched profile revisions, ineligible Hosts, missing mutation/root access, empty context budgets and missing, expired, future-dated or non-enforcing control claims. Filesystem, cancellation and completion authority controls are mandatory. Network, credential and process grants require corresponding enforcing claims. Observed, emulated and unsupported controls never satisfy mandatory controls. Authentication evidence resolution, entitlement checking and adapter-specific strongest-mechanism qualification are pending integrations; a reference to such evidence does not verify it.

Dispatch snapshots pin the Root, Change Stream and Task Contract as well as Task identity/revision. Starting a dispatch rechecks these values and enforcement expiration at command time, and rejects start times before compilation. Deserializing a dispatch revalidates its invariant-bearing contract. Task and Change Stream deserialization replay the recorded commands and reject state, revisions or validation stamps inconsistent with that replay. This prevents structural transition bypass through edited snapshots; it does not authenticate an invented but internally consistent history.

`AgentRuntimeAdapterId` and `InferenceProviderAdapterId` differ structurally. `RuntimeProfile`, `ProviderConnection`, `CredentialReference` and `BillingEntitlement` are separate records. Credential references contain vault locators only, never secret values. Models and provider connections do not acquire canonical Task authority. `NATIVE_SYMBIOTE` and `EXTERNAL_HARNESS` share the same task transition and completion implementation.

**Trust boundary:** callers must resolve authenticated authoritative records before invoking compilation or transitions. Deserialized records, Host IDs, permission snapshots, enforcement claims, verification runs and evidence are not authentication. The future Host must authenticate actors, fetch current records transactionally, verify artifact/run provenance and reject worker-supplied canonical state. This library does not make a forged serialized Host actor trustworthy. The completion routine validates exact evidence lineage, state and coverage; it does not run the checks, authenticate external CI or inspect artifact contents.

## Task and Change Stream behavior

Tasks belong to exactly one Project, Root, Role and Change Stream in this slice. `Task::apply` accepts versioned commands and retains successful event history. A worker's report moves Running → CompletionRequested only. The selected Host starts verification; only that Host can request canonical completion. Completion requires a validated stream at the exact current source **and target** object IDs and one passing evidence record per required gate: tests, security, impact, documentation, independent review, delivery and Capability Closure. Missing, duplicate, failed, inconclusive, foreign-dispatch, cross-Project, stale or future-dated evidence is rejected. Independent review names a different Role. No applicability exemption API exists yet; none of these gates can be weakened by a prompt, model or learned method.

```mermaid
stateDiagram-v2
    [*] --> Ready
    Ready --> Running: Host starts compiled Dispatch
    Running --> CompletionRequested: worker reports
    CompletionRequested --> Verifying: Host begins verification
    Verifying --> Completed: exact-source/target evidence covers every gate
    Running --> Interrupted
    CompletionRequested --> Interrupted
    Verifying --> Interrupted
    Running --> Failed
    CompletionRequested --> Failed
    Verifying --> Failed
    Running --> Cancelled
    CompletionRequested --> Cancelled
    Verifying --> Cancelled
    Interrupted --> Ready: Host recovers; new Dispatch required
    Failed --> Ready: Host recovers; new Dispatch required
```

Cancellation is terminal; retry after a failure or interruption requires a new dispatch compiled against the new task revision. Recovery preserves the prior event/dispatch history. A new command with a stale expected revision conflicts; an identical command ID/payload returns the original resulting revision even after later transitions. Reusing its ID with changed payload conflicts. Failed operations do not mutate or enter the success history; the Host must durably audit rejected commands separately. Timestamps must be monotonically nondecreasing. This in-memory event vector is a contract fixture, not a bounded durable journal design.

A Change Stream is separate from Tasks, Chats, Sessions, Worktrees and branches. It records membership, originating chat, optional objective, sessions, branch/worktree, source/base/target and either independent lineage or a pinned stacked parent/head. Stream commands preserve revision/history and enforce Project/Root mutation access. Advance invalidates prior validation. Collision requires explicit reconciliation and new validation. Integration requires exact validated source and target. Cancellation and integration are terminal. A validation command records the Host's already-verified run identity; invoking it is not execution proof.

```mermaid
stateDiagram-v2
    [*] --> Active
    Active --> NeedsRevalidation: advance source or target
    NeedsRevalidation --> Validated: Host validates exact pair
    Active --> Validated: Host validates exact pair
    Validated --> NeedsRevalidation: advance source or target
    Active --> Collided
    NeedsRevalidation --> Collided
    Validated --> Collided
    Collided --> NeedsRevalidation: explicit reconcile
    Validated --> Integrated: exact pair
    Active --> Cancelled
    NeedsRevalidation --> Cancelled
    Validated --> Cancelled
    Collided --> Cancelled
```

These records do not allocate worktrees, check branch validity, detect semantic collisions, enforce dependency coherence or guarantee two streams cannot claim the same worktree. The future authoritative collection/storage transaction must enforce those cross-record invariants. A stacked parent identity is queryable, but transitive cycle detection and parent movement propagation are pending. Contract tests explicitly model separate worktree/chat identities without claiming physical isolation.

## Ontology coverage and pending acceptance

The table accounts for the required nouns without disguising unimplemented entities as arbitrary JSON. An ID-only entry is **not** a delivered entity schema. Every noun now has one canonical owner, held as data in `src/ontology.rs` and checked rather than reviewed, and the records carry identity, lineage and archival semantics instead of being arbitrary JSON.

| Nouns | Delivered representation | Remaining work |
| --- | --- | --- |
| Project, roots/repos, Role | `Project`, `Root`, `Role` | Project/Role mutation, archival/deletion enforcement, root ownership and uniqueness across records |
| Binding, runtime contract, Dispatch | `WorkforceBinding`, `WorkforceRuntimeContract`, `Dispatch` | Full qualification inputs, required artifact contracts, MCP requirements and production compiler integrations |
| Runtime Profile/account/config identity, provider, credentials, entitlement | Distinct profile/provider/credential/billing records and adapter IDs | Verified account/entitlement resolution, native projection ownership and secret leases |
| Task, Change Stream | Executable pure records/transitions, independent/stacked provenance, validation/collision history | Dependencies/coherence, atomic cross-stream collision/integration decisions and physical worktree allocation |
| Session, Chat, Worktree, branch | `Session`; `Chat` (origin, mutating, its own stream) and `Worktree` (root, branch, path, state, stream) as records; branch is external metadata | Foreign/native session lifecycle APIs and physical worktree allocation |
| Artifacts, verification runs, evidence | `Artifact`; `VerificationRun` (task/dispatch/stream, exact source and target, outcome, timestamps); `VerificationEvidence` citing a run | Authenticated provenance and content-address verification |
| Developer Knowledge/documentation | `KnowledgeClaim` with epistemic status, **confidence**, freshness, evidence/ancestry, audience, anchors and projections; `Projection` names the claim it renders and cannot carry its facts | Publication record, closure/remediation behavior and rejection of circular ancestry |
| User, Host, Fabric, clients/controllers, device, pairing, transport | `User`, `Fabric`, `Device`, `Client` (with advertised capabilities and state), `ControllerSession`, `Pairing` (transport, expiry, revocation), plus `Host` | Execution/service placement, controller authentication and real pairing transport semantics |
| Environments, Harness Drivers/installations, models | `Environment`, `HarnessDriver`, `Installation` (version, path, discovery source) and `Model`, each with state | Compatibility/version resolution and driver/installation lifecycle automation |
| Requests, objectives, capabilities, dependencies | `Request` (the received utterance), `Objective` (accepted intent), `Capability` (with required controls and qualification state); dependency edges and lifecycle | Dependency coherence across streams and cross-record completion semantics |
| Requirements, decisions, ambiguities, diagnostics, annotations, plugins, releases | `Requirement`, `Decision`, `Ambiguity`, `Diagnostic`, `Annotation` (typed target), `Plugin`, `ReleaseTarget`, each with ownership, cardinality and lifecycle | Decision governance and change propagation (#173); per-system plugin trust and release gating |
| GoalRun, NativeExecution/delegation, ExecutionEpisode | Separate records and explicit parent/goal/dispatch/model/access/budget references | Aggregate budget/cancellation enforcement, child authority validation and episode recovery |
| Learned method, experiment | Separate opt-in method and evidence/sample records | Controlled promotion/rollback, measurements/cost denominators and experiment validity enforcement |
| Workforce Protocol, Role Operating Contract, Task Contract, Context Bundle | Separate revision-bearing references; context policy/budget | Full contract content schemas and canonical compilation owners' integrations |
| Native config projection, ephemeral secret lease | `ConfigProjection` (runtime profile, locator, state) and `SecretLease` (credential reference, expiry, state; never a secret value) | Reconciliation of drifted runtime config and lease issuance/rotation |
| External GitHub/harness/CI/deployment/source/publication/remote Host identifiers | `ExternalReference` with explicit system | Per-system locator validation; mutable external references never replace internal identity |

## The vocabulary and its published artifact

Every noun above maps to exactly one canonical entity or value object, held once in `VOCABULARY` with the identity that identifies it and the one type that owns it. The claim is machine-checked rather than asserted: `problems` refuses a noun with two owners, a type that owns two nouns, an identity that identifies two nouns, an identity this crate does not declare, a declared identity with no owning noun, an owner or lifecycle type the published schema does not define, an entity the envelope cannot carry, and an entity with no identity of its own. `tests/ontology.rs` holds the real vocabulary to all of it and asserts the rules are not vacuous (60+ nouns, 55+ identities, 150+ published names, 50+ wire records), and its companion test exercises each refusal by name on inputs that break it.

The published artifact is `docs/contracts/domain-ontology.v1.schema.json`, emitted by `cargo run -p symbiote-domain --example ontology_schema`, guarded by `--write`/`--check` and asserted against the tree by the suite, so the document cannot drift from the types this crate compiles. One definition per named type is held once, with an index from canonical names to those definitions and the list of records `DomainEnvelope` carries; the host CLI's `docs/contracts/schemas/` directory stays owned by `symbiote schema --write` and is not where this artifact belongs.

Two boundaries are stated where a reader meets them rather than left to read as more than they are. Eighteen records that predate this change carry lineage in record-specific fields (`Task.history`, `VerificationEvidence.verified_by`, `WorkforceRuntimeContract.compiled_at` and so on) instead of the shared `Provenance` value object; the suite lists them exactly, fails if a record gains shared lineage while still listed, and routes unifying them to the persistent store that decides the surviving record shape. `WorkforceRuntimeContract` is the one record whose identity is its owning Dispatch's `contract_id` rather than a field of its own, because it is immutable-per-Dispatch compiled state.

The remaining #36 criteria are owned elsewhere and are not claimed here: schema migration rules by [#43](https://github.com/RepairYourTech/SymbioteIDE/issues/43), the same entities serialized by desktop, Host, mobile, plugin, adapter and documentation consumers by [#182](https://github.com/RepairYourTech/SymbioteIDE/issues/182), multi-Host Fabric semantics by [#272](https://github.com/RepairYourTech/SymbioteIDE/issues/272), external-only/native-only/hybrid conformance paths by [#464](https://github.com/RepairYourTech/SymbioteIDE/issues/464), and gate integrity against metrics, prompts, native completions and learning candidates by [#449](https://github.com/RepairYourTech/SymbioteIDE/issues/449) — this layer checks only the structural half: a dispatch weaker than its binding requires is refused, and a learned method carries no field that could grant, relax or exempt a gate. Cross-record audit transactions, documentation authority/closure enforcement, physical stream isolation and runtime/Host integration also remain pending. #170 conformance is demonstrated here for stable Role identity, deterministic bounded snapshot compilation, explicit enforcement strength and rejection of worker bypass of canonical completion.

## Evolution, verification and applicability

`DomainEnvelope.schema_version` is `1`; call `validate_version` before use. Records deny unknown fields. Breaking field, meaning or transition changes require a new schema version and an explicit tested converter; do not deserialize future versions into old authority models or silently drop fields. Version 1 has no prior persisted format to migrate and no persistent implementation may depend on this partial ontology without completing its required contracts. Rollback is removal/reversion of the crate and its consumers; there is no database migration or production state change in this slice.

The contract suite covers native/external canonical parity, deterministic compilation and stable Role identity, unsupported/expired controls, denied permissions, profile/Project/Host mismatches, worker and foreign-Host denial, missing/stale/failed/duplicate evidence, independent review, optimistic races and retry conflicts, interruption/failure recovery and cancellation, revalidation/collision/integration, stream lineage, serialization and schema roots. The ontology suite adds the vocabulary's one-owner rules and their refusals by name, the published artifact's drift guard, the separation of runtime owner/provider/authentication/billing/goal/delegation/episode/learning/experiment identities, the staffing records' distinctness, a new mutating chat opening its own stream and worktree, claim confidence surviving its projections, external locators changing nothing, and the criteria this layer routes elsewhere. It does not count fixture claims as actual verification runs. Real consumer conformance, durable crash/replay recovery, no-Codex native execution, billing isolation and multi-Host authority tests remain pending.

Security/privacy applicability is required at the authenticated Host boundary; schema-level denied paths are covered here, secrets remain references. UI accessibility is not applicable to this headless pure-contract slice; workbench acceptance remains required. Cross-platform wire formats are portable but only the current Linux Rust toolchain was exercised. Resource/performance evidence, bounded persistent histories and production concurrency are pending; no efficiency claim is made. This code introduces no telemetry, process launch, network request or credential access.
