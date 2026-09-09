# Project Team foundation (#200)

Team configuration is versioned canonical Project state, persisted by the Host.
It references existing Roles rather than defining another Role identity or
embedding runtime/profile/model credentials. Configure custom Role names during
Project registration. Operating policies express responsibilities, task domains,
permission ceilings, context/tool/skill/execution policy references, independent
reviewers and explicit-consent fallback intent. Templates provide starting
policies without imposing an organization chart.

A configured Team requires a Lead and a distinct general execution Role, with
independent review coverage. Validation rejects dangling references, fallback
cycles, self-review, duplicate members and privilege escalation beyond the
declared Project ceiling. Store validation ties Roles, Lead and roots to the
registered Project. These declarations are not effective execution permissions;
Host preflight must separately resolve current access and enforce it.

## Durable Host operations

Protocol supports `replace_team` and `get_team`, introduced in v1.3; v1.6 routes work to Team Roles deterministically (see [Role routing](role-resolution.md)). A replacement requires the
separate authenticated `ManageTeam` Project grant. Read access does not confer
management. Actor and timestamp come from the Host, never client JSON.

Initial replacement uses `expected_revision: null` and Team revision 0. Updates
require the current revision and supply exactly its successor. Exact retries
retain the command ID, recover the original timestamp and return the original
receipt. Concurrent writers cannot both replace the same revision. Every accepted
replacement journals its full configuration and authenticated attribution.
Project switching and daemon restart preserve the configuration and history.

SQLite schema v4 adds `team_configurations`. Migration preserves existing journal
bytes and audits materialized Team state against replay before committing. Legacy
Projects have no configured Team; they are not silently staffed. No Roles or
historical Dispatch records are rewritten. Keep a consistent backup before
upgrading; rollback uses a complete prior backup and matching older binary, not
an in-place schema downgrade. Upgrade daemon and CLI together because strict
older protocol clients cannot decode Team events.

## Demonstration and verification

Build `cargo build -p symbiote-host --bins`, start `symbioted --state-dir` using a
private directory, and send the JSON files in `fixtures/project-team` through
`symbiote --state-dir ... raw <file>` (typed drafts): `register.json`, `configure.json`, `read.json`.
Restart the daemon and resend `configure.json`; the receipt must be replayed and
`read.json` must return the same Team. `shutdown.json` stops the demo daemon.

Tests cover structural rejection, permissions, revisions, concurrent updates,
replay, tampered materialized state, cross-Project roots and restart. The daemon
test kills the process between accepted configuration and retry. Portable data
contains symbolic policy references, not local configuration paths or secrets.
No new UI is included, so visual/accessibility acceptance remains with the
workbench; Linux is the actual daemon integration target, with other platforms
still pending their existing transport/platform work.

## Pending acceptance

Valid structure does not mean a qualified workforce. No worker launches from
these operations. #203 must qualify and persist native/external runtime bindings;
#204 resolves Roles and fallback; #269 supplies effective assignment access;
the dispatch/scheduler owners enforce launch and completion gates. Referenced
policies need resolution before activation. Native-only, external-only and mixed
real coding workflows remain unproven. Runtime changes must preserve Role IDs,
and every fallback must requalify resources/access and receive required consent.
Broad #200 remains open until these integrations and UI acceptance are proven.
