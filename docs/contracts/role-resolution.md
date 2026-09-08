# Deterministic Role routing foundation

Canonical owner: [#204](https://github.com/RepairYourTech/SymbioteIDE/issues/204), depending on #189 and #203. `symbiote-workforce::routing` implements the first dependency-ready slice: deterministic validation of explicit Role requests and deterministic classification of untyped work against the durable [Project Team](project-team.md). This is not full #204 acceptance. Graph-derived splitting (#241), learned preferences, runtime-aware fallback execution and quota/rate-limit consumption remain pending; none of them is simulated here.

## Deterministic routing contract

`resolve_route(team, request)` is a pure function: the same recorded Team structure and inputs always yield the same decision. It validates the Team, the request bounds (at most 16 domains, each a Team task-domain reference: 1–128 ASCII alphanumeric or `._-` characters) and Project identity before routing.

- **Explicit requests are honored first.** When the Lead or user supplies `requested`, that exact Role resolves when it is a current Team member, regardless of domain coverage. The requested Role is preserved separately from the resolution, including in no-route records. Learned preferences never override an explicit assignment; none exist yet.
- **Classification never routes to the Lead.** The Lead orchestrates; when Lead execution is intended it is requested explicitly and recorded as `explicit_request`. Classification considers only non-lead members holding `ExecuteProcess`, in lexicographic Role order: a member whose `task_domains` cover every requested domain resolves with `exact_domain_match` and `deterministic` confidence; otherwise the first member with partial coverage resolves with `partial_domain_match` and `heuristic` confidence.
- **No-route is a diagnosis, never a substitution.** Unknown requested Roles produce `requested_role_not_member`; uncovered domains produce `no_executable_role` naming the missing domains. No fallback Role is chosen silently.

A resolved Role is a Team policy statement only. It is not a qualified runtime binding: binding readiness, provider/model entitlement, Host capacity, credential access and enforcement claims remain separate downstream gates (see [workforce bindings](workforce-bindings.md)).

## Durable decisions

A `RouteDecision` records `project_id`, the canonical `work_id`, the `requested` Role separately from the `resolved` Role, the `reason`, the `confidence`, the echoed domain inputs and, when routing failed, the `diagnosis`. Exactly one of `resolved`+`reason`+`confidence` or `diagnosis` is present; schema version 1 rejects mixed states.

Protocol v1.6 adds:

- `resolve_route` (read permission): pure preview of the deterministic decision. The Host resolves from authoritative storage; clients never supply a decision.
- `record_route` (ManageWork): resolves and persists the decision through the store. `command_id` keeps exact replay semantics: an identical retry returns the original receipt.
- `get_route` (read permission): returns the latest decision for a work item.

SQLite schema v6 adds `work_routes`, the latest decision per canonical work item, with a nullable resolved-Role foreign key. Re-routing the same work item supersedes the stored decision; the journal (`work_routed` events, revision 0) retains the full history so manual reassignment keeps its provenance. Replay validation re-derives each decision against the work item and Team state at that journal position, and `work_routes` must replay exactly to the stored rows. Recording requires the referenced work item to exist in the decision's Project and the resolved Role to be a current Team member; there is a bounded cap on distinct routed work items (`resource_exhausted` beyond it).

The work item's owning `role_id` is never mutated by routing; reassignment of canonical work identity remains a revision-gated work edit. Route decisions record execution intent for later dispatch compilation.

## Evidence and remaining acceptance

Covered by unit tests (determinism, explicit-first, lead exclusion, capability gating, no-route diagnosis, bounds), store tests (idempotent recording, reassignment, restart replay, invalid work/member/shape rejection) and the daemon fixture flow (`fixtures/role-resolution/`: register, team, work, classify/explicit/no-route resolution, record, reassignment, restart, readback).

Pending #204 acceptance: classifier/recommender consumed from Project domain/graph evidence rather than caller-supplied domains; multi-Role splitting of mixed work; policy-ordered fallback execution (ask-before-fallback, auto, queue-until-primary, block); runtime availability, quota/rate-limit and model-availability consumption without guessed equivalence; integration with dispatch compilation (#205). Same-recorded-input determinism and explicit-first precedence are the load-bearing invariants those extensions must preserve.

Applicability: security/privacy — routing decisions expose Team structure only to principals already authorized for the Project; no credentials, model accounts or Host paths are included. Accessibility is not applicable to a headless routing library. Performance is bounded by input limits, not benchmarked. Cross-platform: the routing contract is OS-independent; storage behavior is evidenced on Linux only.
