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
gates. It consumes existing Runtime SDK and Host Pulse contracts rather than
inventing successful runtime observations. The current Host supplies its real
pulse and no runtime descriptor, so missing runtime evidence remains visible.
Physical RAM/CPU totals do not satisfy required effective-capacity checks.

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
using `symbiote --state-dir PRIVATE_DIRECTORY request`. Restart and retry the same
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
