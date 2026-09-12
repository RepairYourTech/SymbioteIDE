# Durable workforce binding intent (#203)

`BindingConfiguration` extends the existing canonical `WorkforceBinding` identity
with a pinned Team revision, explicit primary/fallback staffing candidates and
operating policy references. It does not define a second Role or binding ID.
Each candidate records its runtime profile, provider/account references, model,
eligible Hosts, symbolic configuration identity, access, tools, skills, secret
scopes, context and total resource limits. Native and external runtime choices
remain separate from inference-provider identity.

The primary candidate must match the canonical binding. Fallbacks carry complete
independent scopes and require explicit consent; there is no automatic fallback
execution. Policies reference environment, worktree, verification, documentation,
artifact, MCP and escalation contracts. Root effort is explicit; local children
remain disabled until their enforcement integration exists. No credential value,
local profile path, install action or spending authority is included.

## Validation and persistence

Configuration is bounded to 32 KiB and validates canonical Project/Role/profile
relationships, required enforcement controls and Team permission ceilings.
Runtime changes do not change Role identity. A single profile may be referenced
by different Roles, but each binding retains its own effective-intent scopes.
Sharing an identity is not proof of runtime isolation.

Protocol v1.5 adds `replace_binding`, `get_binding` and `get_binding_readiness`; v1.6 adds [Role routing](role-resolution.md) operations that consume validated Teams.
Management requires a separate `ManageBindings` Project grant. Ordinary Team/work
management and read access do not imply that grant. The Host supplies actor and
timestamp; clients cannot inject authority. Readiness additionally requires the
local Host owner because it consumes Host resource observations.

SQLite schema v5 adds binding state with immutable binding/Project/Role identity,
one binding identity per Project Role, revision checks, transactional journal
snapshots and exact retry receipts. A Team update can make a binding stale without
destroying its desired state. Replay validates each binding against the Team at
that point in history, not against the latest Team. Subsequent replacement and
readiness checks require the current Team revision. Historical Dispatch records
remain unchanged.

Migration audits existing state before commit. Back up the complete consistent
Host state before upgrading. Rollback restores a matching earlier backup/binary;
there is no in-place database downgrade. Upgrade CLI and daemon together for the
new strict protocol version. Existing Projects are not automatically bound.

## Readiness is not activation

The read-only assessment separates prerequisite checks from pending activation
gates, and reports five: the current Team, Host capacity, the provider
registration the candidate profile names, runtime capabilities and runtime
resources. It consumes existing Runtime SDK and Host Pulse contracts rather
than inventing successful observations.

**Absence and insufficiency are different results.** Every prerequisite is
classified the same way: `missing_observation` means the Host holds no current
observation of the fact the prerequisite names, and `rejected` means it holds
one and the fact does not meet the requirement. A prerequisite no Host could
observe is never presented as a judgement against it, and no prerequisite is
`satisfied` by default. A Host sampling nothing (`--no-telemetry`), a sample
that expired, a resource the Host did not observe, a runtime the operator did
not declare for that profile, an unreadable cgroup, an expired runtime evidence
window and a capability the observation itself marks unknown all report
`missing_observation` — each is the absence of a current observation, not a
verdict. An ineligible Host, an observed capacity below the profile's limit, a
runtime observed on another Host, a runtime whose observed capabilities,
controls, tools or context bounds do not meet the candidate's requirements, and
a registry refusal all report `rejected`.

`provider_registration` is the registry's own resolution of the candidate
profile's registration, through the same validation a start and a run apply
([provider registry](provider-registry.md)), and it carries the registry's
refusal name in `provider_refusal` whenever it refuses. The pre-dispatch report
and the execution boundary therefore cannot disagree about whether a dispatch
can run.

Host capacity is judged against the effective memory the Host actually
observed from the process's own cgroup, not against the machine's totals: with
no enforced cgroup ceiling the effective availability is the machine's observed
availability, and a Host that cannot read its hierarchy reports unknown rather
than substituting physical totals ([Host inventory](host-inventory.md)). A
profile whose memory limit exceeds the observed effective availability is
`rejected`; one the Host has not observed at all is `missing_observation`.

Runtime capabilities and resources consume the Host's observation of the
operator's declared runtime for that exact profile (see
[runtime SDK](runtime-sdk.md)): the declaration states which runtime the
operator provides and what it supports, and the Host stamps its own identity
and a bounded evidence window onto the observation. With no matching
declaration the Host observes no runtime, so both prerequisites remain
`missing_observation` rather than passing on an assumption.

Environment resolution, effective access/resource consent, provider billing and
model discovery, resource reservations, budget enforcement and Dispatch snapshots
remain required integrations. Even a contract fixture whose observed prerequisite
checks pass is only ready for further preflight, never qualified for execution.
Workers cannot obtain credentials, mutate source or request paid fallback through
these configuration/readiness operations.

## Demonstration and remaining acceptance

With `symbioted --state-dir PRIVATE_DIRECTORY` running, send the Project Team
`register.json` and `configure.json` fixtures, followed by
`fixtures/workforce-bindings/configure.json`, `read.json` and `readiness.json`,
using `symbiote --state-dir PRIVATE_DIRECTORY raw <configure.json>`. Restart and retry the same
configuration command to recover its receipt. The demo references an explicitly
unresolved Host/profile and must report incomplete readiness.

Tests cover revision races, exact retries, Project isolation, Team drift,
historical replay, tampered state, native/external intent and rejected fallback
scopes. These are contract/recovery tests, not live authenticated runtime proof.
No UI is included; accessibility acceptance remains with the workbench. Linux is
the tested daemon transport; other platforms and remote enrollment remain pending.

#203 stays open for fully qualified native/external/mixed staffing, provider and
resource discovery, fallback re-resolution, runtime isolation, Dispatch integration
and actual coding work. Canonical policy/dispatch owners supply those integrations;
this crate does not replace them with another implementation.
