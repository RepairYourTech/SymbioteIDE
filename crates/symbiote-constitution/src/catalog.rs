//! The ledger's inventory: what the invariants are, not how they are checked.
//!
//! Each entry is a compile-time constant of `&'static` data, so no runtime input
//! can weaken, add or skip an invariant. The clauses are quoted from the
//! constitution verbatim, so an edit to the normative sentence and an edit to
//! the ledger cannot drift apart.

/// A fact of the repository tree rather than of a document or a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fact {
    /// No manifest declares Electron, whose use the constitution forbids.
    NoElectron,
    /// No Go source or module exists: Go is not a second core language.
    NoGoCore,
    /// The workspace forbids unsafe Rust and every crate inherits that lint.
    ForbidUnsafeRust,
    /// The workbench is strict TypeScript and its build type-checks the tree.
    StrictTypeScript,
}

impl Fact {
    pub fn name(self) -> &'static str {
        match self {
            Fact::NoElectron => "no_electron",
            Fact::NoGoCore => "no_go_core",
            Fact::ForbidUnsafeRust => "forbid_unsafe_rust",
            Fact::StrictTypeScript => "strict_typescript",
        }
    }
}

/// One non-negotiable invariant with the evidence that must exist for it to
/// hold. The fields are `&'static` so the catalog is a compile-time constant
/// that no runtime input can weaken.
#[derive(Debug, Clone, Copy)]
pub struct Invariant {
    /// Stable identifier quoted by the report and by `docs/contracts/constitution.md`.
    pub id: &'static str,
    /// The requirement this invariant satisfies, quoted from #170 or the constitution.
    pub requirement: &'static str,
    /// The invariant in one normative sentence.
    pub statement: &'static str,
    /// Clauses that must appear verbatim in the constitution.
    pub document: &'static [&'static str],
    /// Authorizations that must not appear; the constitution's non-goals are
    /// machine-enforced as absences, since prose that permits a forbidden
    /// choice reads as policy.
    pub forbidden: &'static [&'static str],
    /// `relative/path::test_name` bindings that the workspace test run executes.
    pub tests: &'static [&'static str],
    /// Facts of the tree that must hold.
    pub facts: &'static [Fact],
    /// Canonical issues that own the integration this invariant does not implement.
    pub owners: &'static [u64],
}

/// Normative sentences the constitution states once and more than one invariant
/// depends on. Held once, so each clause has a single owner here as well as in
/// the document: an entry that needs one names it rather than restating it.
const WORKER_REQUESTS_COMPLETION: &str =
    "The worker requests completion; Symbiote verifies completion.";
const NO_WEAKER_GATE: &str = "No metric, prompt, native completion claim, goal or learned method weakens security, impact, documentation, independent review, delivery or Capability Closure gates.";
const DUAL_PROJECTIONS: &str =
    "Human documentation and compact agent context are projections of the same knowledge.";
const MATERIAL_MUTATION_ISSUE_LINKED: &str = "Material mutation belongs to an issue-linked Change Stream and appropriately isolated branch/worktree.";
const RUNTIME_CHECKS_REMAIN_PENDING: &str = "Runtime failure, interruption, recovery, concurrency and permission checks remain pending until real mechanisms exist; mocks cannot pass later integration acceptance.";
const NATIVE_WORKERS_CANNOT_WRITE_CANONICAL_STATE: &str =
    "Native workers cannot directly write privileged canonical state.";

/// The non-negotiable invariants of the Symbiote constitution, in the order the
/// constitution and #170 name them. Every entry is checked by
/// [`evaluate`](crate::report::evaluate); an entry that stops being checked
/// fails the conformance suite rather than passing quietly.
///
/// The `document` clauses are quoted from the constitution verbatim, so the text
/// and the machine checks cannot drift apart: editing the normative sentence
/// without editing the ledger fails the build. `forbidden` lists authorizations
/// the non-goals exclude, which is how a permission that only ever lived as
/// prose becomes an absence the build can see.
pub const INVARIANTS: &[Invariant] = &[
    Invariant {
        id: "CN-01",
        requirement: "#170: the constitution defines the client/firm model and the selected-Lead versus Symbiote-control-plane boundary.",
        statement: "Symbiote is the firm and the human is its client; the selected conversational Lead owns no canonical state, and only Symbiote completes work.",
        document: &[
            "is an open-source AI software-development firm",
            "The human is its client and supplies goals, constraints, feedback and approvals.",
            "Repeated model selection and manual terminal orchestration are not the product model.",
            "it does not own canonical state, scheduling, policy, context, documentation, evidence or completion.",
            WORKER_REQUESTS_COMPLETION,
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-protocol/tests/contracts.rs::work_wire_never_accepts_host_completion_or_claimed_authority",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-02",
        requirement: "#170: Role identity remains stable when harness, model, account or Host changes.",
        statement: "A Role outlives its runtime: re-staffing changes the versioned Binding, never the Role.",
        document: &[
            "Keep these identities separate: stable Role;",
            "A Role survives changes to runtime, account, model or Host.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/contracts.rs::compilation_is_deterministic_and_role_survives_restaffing",
            "crates/symbiote-runtime-sdk/tests/provider.rs::mixed_runtime_profiles_roundtrip_without_changing_role_identity",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-03",
        requirement: "#170: WorkforceBinding and WorkforceRuntimeContract are defined with deterministic compilation and provenance.",
        statement: "Resolution follows Task to Role to Workforce Binding to Harness/runtime Profile to Eligible Host, and equal pinned inputs compile to an equal versioned contract.",
        document: &[
            "The versioned Workforce Runtime Contract is the deterministic, provenance-bearing compilation of resolved inputs for a dispatch",
            "Resolution follows `Task → Role → Workforce Binding → Harness/runtime Profile → Eligible Host`.",
            "Identical pinned inputs must produce an equivalent contract; unsupported or ambiguous mappings fail explicitly.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/contracts.rs::compilation_is_deterministic_and_role_survives_restaffing",
            "crates/symbiote-workforce/tests/contracts.rs::roundtrip_preserves_canonical_identity_and_strict_metadata",
            "crates/symbiote-domain/tests/contracts.rs::typed_schemas_roundtrip_dispatch_and_reject_unknown_fields_and_versions",
        ],
        facts: &[],
        owners: &[36, 184, 203, 205],
    },
    Invariant {
        id: "CN-04",
        requirement: "#170: adapter enforcement strength is explicit, and unsupported control cannot masquerade as native enforcement.",
        statement: "Every harness packs its control strength as native, wrapped, externally observed, emulated or unsupported, and only demonstrated native or Host mechanisms authorize.",
        document: &[
            "Every Harness Pack declares control strength as native, wrapped/Host-enforced, externally observed, emulated or unsupported, using the strongest verified supported mechanism.",
            "Only demonstrated native or Host mechanisms enforce policy; observation is never prevention.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/contracts.rs::observed_emulated_unsupported_and_expired_controls_cannot_authorize",
            "crates/symbiote-domain/tests/contracts.rs::binding_enforcement_floor_refuses_weaker_host_claims_at_assignment",
            "crates/symbiote-runtime-sdk/tests/adapter.rs::evidence_expiry_version_drift_and_observed_controls_reject_qualification",
        ],
        facts: &[],
        owners: &[184, 203, 205],
    },
    Invariant {
        id: "CN-05",
        requirement: "#170: a worker reporting done cannot complete canonical work without Symbiote gates.",
        statement: "A worker completion report is advisory; canonical completion requires the Host's exact-source and exact-target evidence for every gate.",
        document: &[
            "are advisory foreign execution evidence.",
            WORKER_REQUESTS_COMPLETION,
            NO_WEAKER_GATE,
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/contracts.rs::worker_done_is_advisory_and_worker_cannot_begin_or_complete_verification",
            "crates/symbiote-domain/tests/work.rs::exact_gate_and_acceptance_evidence_completes_only_through_host",
            "crates/symbiote-runtime-sdk/tests/events.rs::tool_lifecycle_report_and_exit_never_create_canonical_completion",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-06",
        requirement: "#170: the constitution states \"the process is code; expertise is pluggable\" and prevents core methodology from being delegated to giant skills or prompts.",
        statement: "Core methodology and control are native code; skills and instructions may carry only specialist expertise.",
        document: &[
            "The process is code; expertise is pluggable.",
            "Skills and instructions provide expertise and guidance.",
            "Core methodology and control cannot depend on a giant skill/prompt bundle; skills carry specialist expertise.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-config/tests/environment.rs::required_constraints_and_core_policy_cannot_be_overridden",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-07",
        requirement: "#170: native harness configuration and session state are projected/foreign state that stays independently usable where upstream supports it.",
        statement: "`.claude/`, `.codex/`, `.pi/` and `.opencode/` are projections of canonical intent; unknown user-owned content survives reconciliation.",
        document: &[
            "and equivalents are projected/runtime state, with canonical desired intent in Symbiote.",
            "Preserve unknown user-owned content and independent upstream use where supported.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-config/tests/contracts.rs::partial_manifest_opens_without_harness_or_authentication",
            "crates/symbiote-projection/tests/preparation.rs::resolved_environment_prepares_comments_and_unmanaged_current_edits",
        ],
        facts: &[],
        owners: &[187, 214],
    },
    Invariant {
        id: "CN-08",
        requirement: "#170: Living Documentation with confidence, provenance, freshness, dual projections, comment hygiene, reconstruction uncertainty, debt and closure is constitutional behavior.",
        statement: "Human documentation and agent context are projections of one structured knowledge base whose evidence, freshness and reconstruction status are explicit.",
        document: &[
            "Living Documentation is structured Developer Knowledge with stable identity, evidence, rationale uncertainty, confidence, freshness, audience, ownership and code links.",
            DUAL_PROJECTIONS,
            "Reconstruction distinguishes CONFIRMED, DERIVED, INFERRED and UNRESOLVED knowledge.",
            "Retain local invariant, safety, algorithm and external-constraint comments",
            "Support code↔docs navigation, documentation blast radius, stale/contradicted/debt states, Context Broker use, Documentation Closure and release/capability integration.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-context/src/lib.rs::resolution_carries_prompt_and_structured_guidance_bounded",
        ],
        facts: &[],
        owners: &[29, 30, 31, 32, 33, 34],
    },
    Invariant {
        id: "CN-09",
        requirement: "#170: the constitution locks Linux-first Host architecture, cross-platform clients, System Graph, Context Broker, Deep Guidance, Capability Closure and issue-first delivery.",
        statement: "The Linux-first Host owns canonical state and outlives its clients; the named subsystems are locked as constitutional behavior.",
        document: &[
            "The Linux-first Host owns canonical state; desktop, CLI and cross-platform/remote clients share native execution and Host semantics.",
            "Closing or crashing desktop must leave authorized headless work alive.",
            "The System Graph models source/symbols",
            "The Context Broker supplies the smallest sufficient task/Role/model/permission/Host-aware evidence package",
            "Deep Guidance is native stateful methodology",
            "Capability Closure covers applicable functionality",
            MATERIAL_MUTATION_ISSUE_LINKED,
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-context/src/lib.rs::broker_issues_a_scoped_expiring_lease_on_the_happy_path",
            "crates/symbiote-host/tests/daemon.rs::team_revisions_survive_crash_and_do_not_leak_between_projects",
        ],
        facts: &[],
        owners: &[38, 232, 241],
    },
    Invariant {
        id: "CN-10",
        requirement: "#170: non-goals include no proprietary required model, no forced inference reseller, no editor-fork dependency, no transcript as source of truth, no generated-doc circular authority and no provider-local completion authority.",
        statement: "Each non-goal is an absence the build can see, not a preference a later edit may reverse by writing permission into prose.",
        document: &[
            "Electron is ineligible for the desktop.",
            "Go is not a second core language.",
            "No proprietary required model, forced inference reseller, editor-fork dependency, transcript authority, generated-doc circular authority or provider-local completion authority is permitted.",
            "Use progressive expansion and structured handoffs rather than transcript dumping.",
        ],
        forbidden: &[
            "Electron is eligible",
            "Go is a second core language",
            "A proprietary required model is permitted",
            "The transcript is the source of truth",
            "Generated documentation is authoritative",
        ],
        tests: &[
            "crates/symbiote-runtime-sdk/tests/provider.rs::native_local_endpoint_needs_no_cli_or_secret_but_requires_exact_local_policy",
        ],
        facts: &[Fact::NoElectron, Fact::NoGoCore],
        owners: &[],
    },
    Invariant {
        id: "CN-11",
        requirement: "#170: constitutional changes require ADR-backed downstream impact propagation.",
        statement: "An accepted decision is immutable; a change arrives as a superseding ADR that names the artifacts it invalidates.",
        document: &[
            "Accepted constitutional changes require a superseding ADR with authority, constraints, alternatives, dated/versioned evidence, reversibility, consequences and downstream impact propagation.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-architecture/tests/governance.rs::supersession_invalidates_only_dependent_artifacts",
            "crates/symbiote-architecture/tests/governance.rs::acceptance_requires_new_pin_and_freezes_content",
        ],
        facts: &[Fact::ForbidUnsafeRust],
        owners: &[173],
    },
    Invariant {
        id: "CN-12",
        requirement: "#170: unit and integration tests cover normal, boundary, failure, interruption and recovery behavior appropriate to this issue.",
        statement: "The workspace tests interruption, failure recovery, cancellation and optimistic-retry behavior rather than only the happy path.",
        document: &[RUNTIME_CHECKS_REMAIN_PENDING],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/contracts.rs::interrupted_and_failed_tasks_recover_with_new_dispatch_cancelled_tasks_are_terminal",
            "crates/symbiote-domain/tests/contracts.rs::optimistic_updates_preserve_success_history_and_retries_are_idempotent",
            "crates/symbiote-store/src/tests.rs::process_kill_at_migration_state_journal_and_commit_boundaries_is_atomic",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-13",
        requirement: "#170: public schemas, APIs and configuration are documented, and migration and rollback are included where state changes.",
        statement: "The published schemas are generated artifacts compared against their committed fixtures, and a version change carries a tested converter.",
        document: &[
            "Runtime/provider/platform facts expire independently and require refreshed official evidence.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-host/src/cli_schema.rs::the_committed_fixtures_are_exactly_what_the_builders_emit",
            "crates/symbiote-domain/tests/contracts.rs::typed_schemas_roundtrip_dispatch_and_reject_unknown_fields_and_versions",
            "crates/symbiote-store/src/tests.rs::v1_migration_preserves_existing_records_and_journal",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-14",
        requirement: "#170: the implementation emits observable evidence and actionable errors instead of swallowing unknown or degraded states.",
        statement: "Provenance, uncertainty and unmeasured behavior stay visible, and an unknown is distinct from a zero.",
        document: &[
            "Provenance, uncertainty, coverage and unresolved dynamic boundaries are visible.",
            "approved debt, unknowns and unmeasured behavior remain visible.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-host-inventory/tests/inventory.rs::probe_failures_are_preserved_and_contradictions_rejected",
            "crates/symbiote-host-inventory/tests/inventory.rs::unsupported_platform_never_claims_operating_system_probe_success",
            "crates/symbiote-runtime-sdk/tests/events.rs::usage_unknown_is_distinct_from_zero_and_hidden_children_stay_visible",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-15",
        requirement: "#170: security, privacy, accessibility, performance and cross-platform applicability are explicitly reviewed and marked required, not applicable with rationale, or separately tracked.",
        statement: "Preview is untrusted, worktrees isolate Git mutation rather than the machine, and a platform that was not exercised is recorded as unproven.",
        document: &[
            "Preview applications, including localhost content, are untrusted and receive no workbench/Host authority. An iframe is not an assumed security boundary.",
            "Worktrees isolate Git mutation, not network access, secrets, production services or the filesystem. The threat model and enforcement evidence remain required.",
            NATIVE_WORKERS_CANNOT_WRITE_CANONICAL_STATE,
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-trust/tests/contracts.rs::every_bound_snapshot_dimension_is_checked",
            "crates/symbiote-trust/tests/contracts.rs::expiry_revocation_and_invalid_time_are_denied",
            "crates/symbiote-runtime-sdk/tests/adapter.rs::structured_and_generic_pty_expose_different_guarantees_for_same_dispatch",
        ],
        facts: &[Fact::StrictTypeScript],
        owners: &[191, 218, 38],
    },
    Invariant {
        id: "CN-16",
        requirement: "#170: work is delivered through the repository's issue, Change Stream, branch, PR, independent review and current-head checks workflow.",
        statement: "Delivery is issue-linked, and the repository's own guard refuses a closure stated in a commit message or a title, so a pull-request description is the only channel that can close an issue; whether that description was independently reviewed is a separate gate this layer cannot check.",
        document: &[MATERIAL_MUTATION_ISSUE_LINKED],
        forbidden: &[],
        tests: &[
            "planning/integrity/test_closing_keywords.py::test_a_commit_message_may_not_close_anything_even_in_a_stated_clause",
            "planning/integrity/test_closing_keywords.py::test_a_landed_title_citing_an_issue_number_passes",
        ],
        facts: &[],
        owners: &[387, 394],
    },
    Invariant {
        id: "CN-17",
        requirement: "#170 additional acceptance: schemas distinguish runtime owner, model provider, authentication and billing entitlement, GoalRun, NativeExecution/delegation, ExecutionEpisode, learned method and experiment identities.",
        statement: "The runtime, provider, credential and billing identities are separate records, and the goal, delegation, episode, method and experiment identities are distinct.",
        document: &[
            "AgentRuntimeAdapter and InferenceProviderAdapter are separate; ProviderConnection, CredentialReference and BillingEntitlement are distinct.",
            "Goal execution and delegation are bounded, resumable, permissioned and observable, with root/child lineage and aggregate budgets.",
            "Opt-in scoped learning improves approved methods without changing security, staffing, billing, entitlements or locked decisions silently.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-runtime-sdk/tests/provider.rs::binding_mismatches_expiry_and_billing_fallback_fail_closed",
            "crates/symbiote-runtime-sdk/tests/provider.rs::external_harness_subscription_never_accepts_copied_native_credentials",
            "crates/symbiote-runtime-sdk/tests/adapter.rs::external_full_harness_cannot_claim_native_ownership_or_transport",
        ],
        facts: &[],
        owners: &[184, 203, 460, 449],
    },
    Invariant {
        id: "CN-18",
        requirement: "#170 additional acceptance: the same client request has external-only, native-only and hybrid conformance paths without changing canonical authority.",
        statement: "Native, external and mixed staffing share one canonical completion contract; a fully native Team needs no third-party coding CLI.",
        document: &[
            "External-only, hybrid and fully native configurations preserve the same canonical authority.",
            "Symbiote Agent works without third-party coding CLIs",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/contracts.rs::native_and_external_paths_share_canonical_completion_contract",
            "crates/symbiote-workflow/tests/end_to_end.rs::two_harness_demo_native_and_external_workers_on_one_project_without_leakage",
            "crates/symbiote-runtime-sdk/tests/adapter.rs::native_external_and_mixed_staffing_preserve_role_task_and_stream_identity",
        ],
        facts: &[],
        owners: &[464, 465, 447],
    },
    Invariant {
        id: "CN-19",
        requirement: "#170 additional acceptance: no metric, prompt, native completion or learning candidate can weaken a required security, documentation, impact, review or delivery gate.",
        statement: "A gate is not negotiable by a worker, a prompt or a learned method; incomplete or self-reviewed evidence never advances work.",
        document: &[
            NO_WEAKER_GATE,
            "Children cannot bypass staffing or acquire undelegated authority.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-domain/tests/work.rs::incomplete_stale_failed_and_self_review_evidence_never_advances",
            "crates/symbiote-domain/tests/contracts.rs::completion_rejects_missing_failed_duplicate_stale_and_self_review_evidence",
        ],
        facts: &[],
        owners: &[460, 454],
    },
    Invariant {
        id: "CN-20",
        requirement: "#170 non-negotiable constraint: native runtime extensibility is an enforcement substrate for Symbiote workforce policy, not the canonical location of that policy.",
        statement: "A worker or harness cannot write privileged canonical state; its report is an input the Host judges.",
        document: &[NATIVE_WORKERS_CANNOT_WRITE_CANONICAL_STATE],
        forbidden: &[],
        tests: &[
            "crates/symbiote-protocol/tests/contracts.rs::work_wire_never_accepts_host_completion_or_claimed_authority",
            "crates/symbiote-host/src/runner.rs::runners_refuse_foreign_runtime_kinds",
        ],
        facts: &[],
        owners: &[],
    },
    Invariant {
        id: "CN-21",
        requirement: "#170 non-negotiable constraint: generated documentation is an evidence-backed compression or knowledge layer, not a substitute for executable or approved primary evidence.",
        statement: "Documentation may compress knowledge but cannot certify itself or replace primary evidence.",
        document: &[
            "Generated documentation cannot certify itself or replace approved/executable primary evidence.",
            DUAL_PROJECTIONS,
        ],
        forbidden: &[],
        tests: &[],
        facts: &[],
        owners: &[29, 30, 31, 32, 33, 34],
    },
    Invariant {
        id: "CN-22",
        requirement: "The constitution's own record: #170's named coverage is accounted for, each by the machine check that runs it or the canonical issue that owns the integration.",
        statement: "The document states that its named coverage is accounted for, and every row of its coverage map names the check that runs it or a canonical issue that owns it; pending runtime checks are pending rather than passed.",
        document: &[
            "This section accounts for every coverage item #170 named as open",
            RUNTIME_CHECKS_REMAIN_PENDING,
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-constitution/tests/conformance.rs::the_coverage_map_names_every_invariant_and_owner",
            "crates/symbiote-constitution/tests/conformance.rs::every_coverage_row_names_a_check_or_an_owner",
        ],
        facts: &[],
        owners: &[36, 173, 38, 470],
    },
    Invariant {
        id: "CN-23",
        requirement: "#170 additional acceptance: follow #464, #447, #460, #449, #454 and #470 as concrete implementation owners; the references are not circular prerequisites of this foundational schema.",
        statement: "This record names those owners and routes the work to them; whether their integrations exist is theirs to prove, so the check this layer can make is that the routing is present and complete.",
        document: &[
            "#464, #447, #460, #449, #454 and #470",
            "These owners are integration obligations, not circular prerequisites of this record.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-constitution/tests/conformance.rs::the_routed_owners_are_named_by_the_record",
        ],
        facts: &[],
        owners: &[464, 447, 460, 449, 454, 470],
    },
    Invariant {
        id: "CN-24",
        requirement: "#170 non-negotiable constraint: follow the product constitution and applicable Project, Role and Host policies.",
        statement: "A specification or implementation obeys the constitution and the policies of its Project, Role and Host, and no caller can override a policy it does not name.",
        document: &[
            "Every specification, issue, adapter, workforce binding, documentation subsystem and implementation obeys this constitution and the applicable Project, Role and Host policies.",
        ],
        forbidden: &[],
        tests: &[
            "crates/symbiote-host/src/bin/symbiote/tests.rs::authorization_follows_the_operation_not_the_command_name",
            "crates/symbiote-host/src/bin/symbiote/tests.rs::a_private_policy_authorizes_exactly_the_dangerous_kinds_it_names",
        ],
        facts: &[],
        owners: &[],
    },
];

/// Why an invariant has no executable binding at this layer, for the entries
/// that have none. The conformance suite requires every invariant without a
/// `tests` or `facts` channel to appear here, and every entry here to name such
/// an invariant, so a future entry cannot arrive with nothing to run and no
/// stated reason — and a reason cannot be used as a crutch by an entry that has
/// an executable check.
pub const EXPLANATIONS: &[(&str, &str)] = &[(
    "CN-21",
    "documentation authority is a rule about documents, and no executable artifact in this workspace can certify that a document is not authoritative; the clause checks are the strongest evidence this layer can produce.",
)];
