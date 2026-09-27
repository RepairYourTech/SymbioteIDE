# Local durable Host — #180 / #181 / #43

The workspace now builds `symbioted` and its `symbiote` CLI client. This is the first persistent metadata service, not an agent runtime. It registers canonical Projects, Roots and Roles together, creates ready Tasks with initial Change Streams, returns records and paginates a durable journal. It does not execute commands, call models or claim task completion. Dispatch activation (`run_started_dispatch`) provisions the task's Change Stream worktree from the Root's recorded host placement — reservation verification, base-commit validation against the stream's recorded base, and materialization — before any transport would be built; with the production configuration (no reservation base, no transports) it refuses with typed errors and nothing executes.

## Run

```sh
cargo build --workspace --locked
target/debug/symbioted --state-dir /absolute/private/state-directory
```

`--state-dir` and `--config-dir` are optional. Without them the Host resolves its own directories from the environment, in the order the CLI already applies to a policy — an explicit flag, then this Host's override variable, then the XDG base directory the specification names, then the specification's default under `$HOME`:

| Directory | Flag | Override | XDG base | Default under `$HOME` |
| --- | --- | --- | --- | --- |
| State: the socket, the canonical store, the lock and Host identity | `--state-dir` | `$SYMBIOTE_HOST_STATE_DIR` | `$XDG_STATE_HOME/symbiote` | `.local/state/symbiote` |
| Configuration: where the operator configuration is discovered | `--config-dir` | `$SYMBIOTE_HOST_CONFIG_DIR` | `$XDG_CONFIG_HOME/symbiote` | `.config/symbiote` |

What lives under each resolved directory, by name — the layout this document owns and
`crates/symbiote-host/tests/paths.rs` holds the Host's own declarations to:

| Role | Directory | File name |
| --- | --- | --- |
| Control socket | state directory | `host.sock` |
| Canonical store | state directory | `control.sqlite3` |
| Operator lock | state directory | `host.lock` |
| Runtime inventory | state directory | `runtime-inventory.json` |
| Operator configuration | configuration directory | `operator.json` |

An empty override means unset, exactly as an empty `$SYMBIOTE_CLI_POLICY` does. A relative `XDG_*` value is ignored — the specification's own rule — so the `$HOME` default applies; a relative value in either override variable is refused instead, because ignoring operator intent would run the Host somewhere the operator did not name. With no flag, no override and no absolute `HOME` the Host refuses and names what was missing rather than writing to the working directory. The parent directory must exist. A new final directory is created mode 0700, and a directory the Host cannot create is reported by name rather than as a bare os error; an existing shared, foreign-owned or symlink directory is refused without changing its permissions. A state directory whose socket path would not fit the Linux socket path bound is refused by name before anything binds. With no `--operator-config` the Host looks for the operator configuration named above under the configuration directory, and a file that is not there means no operator provisioning; the cache, log and runtime directories are not resolved, because nothing in this daemon writes to them (see the pending list). The local socket is mode 0600. Each connection uses Linux SO_PEERCRED to establish same-UID ownership; this is an explicit local-owner policy, not remote or Preview authentication. No TCP listener is opened. An exclusive lock prevents a second daemon replacing the live socket; restart replaces only a stale owned socket after acquiring that lock.

Send a request from another terminal:

```sh
target/debug/symbiote --state-dir /absolute/private/state-directory health
```

## Administrative command surface

`symbiote --state-dir DIR <command> [args...]` maps onto typed protocol
operations — `hello`, `health`, `host-pulse`, `runtime-inventory`, `shutdown`,
`get-project`,
`list-projects`, `snapshot`,
`register-project`, `create-work`,
`create-task`, `get-task`, `read-journal`, `prepare-dispatch`,
`get-dispatch-preparation`, `start-prepared-task`, `run-started-dispatch`,
`request-task-completion`, `request-elevation`,
`validate-change-stream`, `begin-task-verification`, `complete-task`,
`integrate-change-stream`,
`scheduling-projection <project_id>`,
`observe-root-placement`,
`replace-team`, `get-team`, `replace-binding`, `record-route`,
`register-provider-connection`, `register-billing-entitlement`,
`register-model-descriptor`,
`get-task-origin`, and `raw <operation.json>` for anything the typed
commands do not cover (the daemon still validates and authorizes every
field). Responses are the daemon's own JSON, pretty-printed. Exit codes
are distinct: 0 success, 1 usage/connection failure, 2 a daemon-refused
command (`Err` response), 3 authorization required — scripts branch on
refusal vs transport failure vs a missing authorization.
`help` lists the command table, marking dangerous commands and naming the
dangerous kinds a policy may pre-authorize. `help` is answered locally and
honors no flags; `schema` is the other local command: it prints the binary's
published envelope and policy JSON Schemas (see [cli.md](cli.md)) — one of
them alone with a selector, `envelope` or `policy` — or regenerates the
committed fixtures with `--write DIR`, or checks a directory against them with
`--check DIR`: with no selector that directory must hold exactly the published
documents, so a missing, edited, extra or renamed entry is named and exits 1,
reading and writing nothing. It needs no daemon, no state directory and no
authorization. `publish-runtime-inventory` is the third local command, and it
is local by design: the [runtime inventory](runtime-discovery.md) has exactly
one wire operation, an owner-only read, so installing a discovery document is
an operator action on this machine rather than something a client asks a daemon
to do. It re-validates the document whole, requires every record to name that
state directory's own Host identity, installs it atomically at mode 0600, and
names its refusal without replacing what the Host already held. `--write` and
`--check` are valid only on `schema`, and `--json`
is not valid on `schema` or `help` (schema's output is machine-readable JSON,
but not the envelope). Every flag is declared per command: the daemon commands
honor the daemon-facing flags, `schema` honors `--write` and `--check`,
`publish-runtime-inventory` honors `--state-dir`, and
`help` honors nothing; `--help`/`-h` is universal, and a help request honors no
other flag, so `--json --help health` is refused exactly as `--json help` is. A
flag a command cannot honor is a usage error that names it, never silently
dropped. Identity
arguments are passed as plain strings and typed on the wire; no caller-supplied
actor identities exist. The interactive/headless coding-agent experience is
#467's surface and shares this client plumbing; this binary carries no model
loop.

### The write commands, and what they leave to `raw`

Registering a Project, creating work and creating a Task are the writes this
client is built around, and each is a command whose arguments are the facts an
operator or a script actually holds — an identity, a name, a contract
revision, a branch — rather than a JSON document somebody assembled by hand:

```sh
symbiote --state-dir DIR register-project symbiote "Symbiote" lead lead-contract@1 root-main
symbiote --state-dir DIR create-work symbiote lead ship-it operational "Ship the beta" "and say what done is"
symbiote --state-dir DIR create-task symbiote task-one root-main lead ship-it coding@1 5117b5736e2e5cc2ff537a3852f8e1e37c6b0a12
```

Each returns the daemon's own receipt, and each is read back by the typed
reads: `get-project`, `snapshot` and `read-journal` after a registration,
`get-task`, `get-task-origin` and `prepare-dispatch` after a Task.

Two spellings recur because two facts do. A contract is written
`<contract_id>@<revision>`: a contract is recorded under an identity *and* the
revision of it, and only the caller knows which revision this Project or Task
is written against. And `<class>` is the classification the domain requires of
an Objective — `maintenance` and `operational` are the two a Task may
originate from, `outcome` the third the domain declares — because that
classification is what decides whether `create-task` may name the work at all.

Everything else the operations accept is derived rather than asked for, and
each derivation is stated here so that nothing in the record is a surprise. A
registered Root names neither a repository nor a host path: placement is a
Host's to record, per Host, once it has observed a checkout. A registered
Project carries its lead Role, named for the identity it is registered under.
A created Task's stream and its chat are named for the Task it belongs to —
`stream-<task_id>` and `chat-<task_id>` — and its worktree and branch are the
Host's own derivation, not the caller's: the Host re-derives both from its
policy seed before it provisions anything and refuses a stream whose recorded
names disagree, so a caller that names its own gets a Task that can never be
dispatched, refused at `run_started_dispatch` with nothing to act on. The
`symbiote` binary asks the Host for that derivation and records exactly what it
returns, so the pair a Task carries is the pair the Host will insist on. The
stream's base is the `<base_commit>` the caller named, which must be a full
commit id that is not the null object id, because the Host checks it against
the repository's real HEAD before it provisions anything; its target is the
null object id, because a Task that has done no work has reached nothing.
A created Objective states its title, its optional description and its class,
and leaves its requirements, constraints, risks, acceptance criteria, budget,
dependencies and external references unstated at the domain's lowest priority,
because a work item is filed here and specified through `raw`.

### Registering a Project's runtime

`observe-root-placement`, `replace-team`, `replace-binding`, `record-route`,
`register-provider-connection`, `register-billing-entitlement` and
`register-model-descriptor` register what a Project runs on, and every argument
they take is a fact the operator holds. Two argument shapes carry more than one
value, and both are owned here rather than left to a document somebody
assembled: a list is comma-separated, and a record is colon-separated in a
stated field order.

```sh
symbiote --state-dir DIR observe-root-placement symbiote root-main "$(symbiote --state-dir DIR host-pulse | jq -r .data.host_id)" /srv/checkout 0
symbiote --state-dir DIR replace-team symbiote lead ctx tools skills bounded \
  'lead:lead_orchestrator:read_root:root:coordination:engineer' \
  'engineer:general_execution:read_root,execute_process,mutate_stream,use_credential:root:coding:lead'
symbiote --state-dir DIR register-provider-connection symbiote openai openai-responses https://api.openai.example/v1 api_credential
symbiote --state-dir DIR register-billing-entitlement symbiote ent-1 openai metered_api proof-1 4102444800000
symbiote --state-dir DIR register-model-descriptor symbiote coding-model openai 32768 4096 tools
symbiote --state-dir DIR record-route symbiote ship-it engineer
```

A `<member>` is `<role_id>:<function>:<grant>,<grant>:<root>,<root>:<domain>,<domain>:<reviewer>,<reviewer>`.
Every member's access is its own rather than a copy of the Team's, because a
Team exists so two Roles can reach different places; the Team's
`access_ceiling` is the union of what its members were granted, so no member
can reach past it. `read_root` is added to every member, because the domain
refuses a member that cannot read the Root — a member is reviewed against the
Roots it can see. A member's `responsibilities` are its `task_domains`: the two
lists are the same words, and spelling them separately would let a Team say a
Role is responsible for work it never routes to.

`register-project` takes its Roots as one comma-separated argument and any
further Roles as `<role_id>:<contract_id>:<revision>`, because a Project
registered with its lead Role alone can never hold a Team: the store refuses a
Team that names a Role the Project does not hold, and the domain refuses a
Team with no execution role. A Project registered from the command line can
therefore be staffed and dispatched from the command line.

### Completing a dispatched task

Three operations close the lifecycle, and a dispatched task could not take any
of them: over the whole workspace the Host constructed exactly one
`TaskAction` — `RequestCompletion` — so a run that finished left its task at
`completion_requested` for the life of the store, with the worker's report on
record and no way for any client to accept it. `validate-change-stream`,
`begin-task-verification` and `complete-task` are those three steps.

```sh
symbiote --state-dir DIR validate-change-stream task1 run-verify
symbiote --state-dir DIR begin-task-verification task1
symbiote --state-dir DIR complete-task task1 lead \
  tests,security,impact,documentation,independent_review,delivery,capability_closure --yes
```

Each is the same authority `start-prepared-task` holds, and each takes only
what its caller actually knows.

`validate-change-stream` names the verification run; it does **not** name the
commit. The Host observes the provisioned worktree itself and validates what it
reads, because a validation of a commit nobody looked at is a validation of
nothing. A stream whose recorded head and target are not what the worktree
holds stays where it was.

`begin-task-verification` takes a task and nothing else. A task with no current
dispatch has no contract whose gates could be covered, and is refused with the
same refusal a run gives for a task not running under its dispatch.

`complete-task` splits the claim in two. The caller says **which gates
passed**; the Host binds each claim to this dispatch, this stream's recorded
head and target, and its own inventory identity, and the domain refuses a
claim that does not cover the contract's required set. That split is the whole
point: a Host that invented the gate set would be filing a runtime-local
success flag as evidence, and a caller that supplied whole evidence rows would
be filing claims the Host never checked. The `independent_review` gate carries
the caller's reviewer, and the domain refuses one that is the Task's own Role.

`integrate-change-stream` is the step after that. A completed task's work is on
a branch in a worktree, and nothing else moves it: without this the canonical
state says the work is done and the repository says otherwise, and the two
disagree for the life of the store. The Host merges, because "the branch is
the stream's" and "the Root is where it lands" are both facts only the Host
holds. The Root must be at the stream's recorded target before the merge and
at its recorded head after, and the merge is `--ff-only` — a merge the Host
would have to resolve is a merge it has no authority to decide.

That last step is what makes the stream's `target` a fact rather than a
placeholder. `create-task` used to record the null Git object id there, on the
reasoning that a task which has done no work has reached nothing — which made
a task created from the command line dispatchable, runnable, completable and
then permanently un-integratable, because a Root can never be "at" a commit
that does not exist. It records the base instead: that is where the Root is
until work lands, and a Task that cannot finish should not be creatable. A
stream that aims somewhere other than its base is created through `raw`.

`complete-task` is classified dangerous, because it ends a task's execution
rather than changing it. The refusals name themselves — a claim that does not
cover the contract arrives as `lifecycle refused the transition
(EvidenceMissing)` and a self-review as `(IndependentReviewRequired)`, not as
the generic `invalid_request` sentence that says neither.

One thing the command line cannot carry is a credential. Every runtime profile
names one, and a native run refuses without it by name, so the registration is
operator authority this client cannot hold: `symbiote init` writes the 0600
configuration and `symbiote add-credential` records one registration in it,
reading the value from standard input so a secret is never an argument, a
process-table entry, or a line in the journal. The binding's member and
candidate both carry `use_credential`, because the broker's gate is what the
grant is for and a lease cannot issue without it. Both steps are local, and
both happen before the daemon is started, because the configuration is read
once (see [cli.md](cli.md) and
[context-credential-resolution.md](context-credential-resolution.md)).

A binding's `schema_version`, its candidate's `config_identity`, the
candidate's limits, `root_effort`, `local_children`, `fallback_consent` and
the `minimum_enforcement` map are all stated by the command and named in its
refusals, and the binding and the candidate it staffs are written from the same
access, context, tools and skills — the domain refuses a configuration where
they differ, and the only way they cannot is by not being written twice. The
seven policy references are named one argument each: a policy document is
something the operator publishes, and a reference this client invented would
name a document that does not exist.

A model whose `max_output_tokens` exceeds its `context_window_tokens` is refused
where both figures are on the command line, rather than as a `context_limit`
from a provider turn long after the descriptor was on record. That check is the
descriptor's own; the incoherence between a binding's `max_input_tokens` and a
model's window spans two records, and is the provider's to refuse.

`raw` is not a lesser path and not deprecated. It is how a Project with more
than one Role, a work item of another kind (a request, a plan, a milestone),
a fully specified Objective, or a Task whose stream carries real lineage is
created: the typed commands cover the writes a person or a script types, and
the daemon still validates and authorizes every field of both.

A command line that cannot be honored is refused before anything is sent, and
the refusal names the argument, the shape it wanted and the value it got — the
way a refused lock names the lock it could not take — with the command's usage
beside it. An argument that is absent, and an argument that is there and
empty, are both refused that way, because no identity, name, title or branch
can be empty; what an argument may *contain* — its length, its characters,
whether it is only whitespace — is the domain's, and comes back as the
daemon's own typed refusal. A surplus argument is named rather than dropped,
and an argument that begins with `--` is a flag unless `--` precedes it, which
ends flag parsing. A command line large enough to exceed the frame bound is
refused as the request it is, naming its size and the bound, rather than as a
daemon that is unreachable: it never asked (see [cli.md](cli.md)).

### Dangerous operations require explicit authorization

Authorization follows the **operation**, not the command name. One table
classifies operation kinds, and both the typed commands and `raw` are
decided by it — so `raw shutdown.json` cannot bypass the gate on
`shutdown`, and a read through `raw` is not over-gated. Dangerous kinds are
the ones that start or end execution or grant capability:
`shutdown`, `start_prepared_task`, `run_started_dispatch`,
`decide_elevation` (the grant; `revoke_elevation` narrows and is not
gated), `record_resource_consent` and `complete_task` — which ends a task's
execution rather than changing it, so it is gated like the operations that
start it. A kind the table does not know
cannot be proven safe, so it fails closed as dangerous. Reads are free;
everything else is a mutation the daemon still authorizes — filing
evidence (`register-project`, `create-work`, `create-task`,
`prepare-dispatch`, `request-task-completion`,
`request-elevation`) is not starting work.

A dangerous operation is sent only when the invocation is explicitly
authorized: `--yes` for this one run, a `yes` typed at the prompt when
stdin is a terminal, or a **policy** that pre-authorizes the operation
kind. Anything else — no tty, no flag, no policy, an empty read, or any
answer other than `yes` — refuses with exit 3 and **no byte reaches the
socket**: the decision is made before the CLI connects, which the
integration test pins with a listener that never sees an accept. The CLI
holds no daemon authority of its own and widens nothing; it only decides
whether to send, and a policy changes only that decision — a command it
authorizes can still be refused by the daemon (exit 2).

#### Authorization policies

`--policy FILE`, or `$SYMBIOTE_CLI_POLICY` when the flag is absent, names a
versioned `symbiote.cli-policy/v1` document that pre-authorizes dangerous
operation kinds for unattended runs:

```json
{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":4102444800000}
```

`authorize` names **operation kinds**, not command names, so `raw` is covered
by the same rule: a policy listing `shutdown` authorizes `shutdown` and
`raw shutdown.json` alike, and authorizes neither when it lists something
else. It may only name kinds this CLI classifies as dangerous — a read or a
mutation is already ungated, so listing one would be a no-op an operator
could mistake for coverage, an unknown kind cannot be classified at all, and
a wildcard is deliberately not a syntax. `expires_at` is an optional
unix-millisecond bound; the expiry instant itself is still authorized and a
lapsed policy grants nothing, with the refusal naming the file and the lapse
rather than looking like a policy that was never configured.

A policy grants authority, so it is held to the Host's own private-file
discipline, enforced at load rather than trusted from convention: a regular
file (a symlink is refused, not followed), owned by this user, with no
group/other permission bits (chmod 600), bounded at 64 KiB. A policy that is
missing, unreadable, insecure, or names an unknown or non-dangerous kind is
an operator error, not a missing authorization: exit 1, nothing sent, and
`policy_invalid` in the `--json` envelope. It is never silently downgraded to
"no policy", because a typo would then quietly discard a pre-authorization
the operator believes is in force. The policy is consulted only for an
operation that is already dangerous and not already authorized by `--yes`, so
a broken policy cannot break `health`, and `--yes` cannot be blocked by one.
A policy is a pre-authorization, never a widening: it cannot grant a
permission the daemon would refuse.

Flags are recognized before or after the command (`symbiote shutdown
--yes` works), `--` ends flag parsing so an argument beginning with dashes
stays expressible, and an unknown flag is a usage error rather than a
silently-dropped token. `--json` prints one machine-readable envelope per
invocation on stdout instead of pretty output (the local `schema` and `help`
commands do not accept it, and `--help`/`-h` is answered from the table; see
[cli.md](cli.md)): `schema` is the versioned
`symbiote.cli/v1`, alongside `command`, `command_id`, `ok`, and either
`result` (the daemon's own body, unmodified) or `error.code` — the daemon's
error code, or the CLI's own `authorization_required` (nothing was sent),
`policy_invalid` (the configured policy could not be honored; nothing was
sent) and `unreachable`. The default output is unchanged, so existing scripts
that read pretty JSON keep working. Minted command IDs carry millisecond,
pid and an in-process sequence, so two invocations cannot share an
idempotency key and have one mistaken for a replay of the other. Both the
envelope and the policy document are published as versioned JSON Schema
fixtures in [cli.md](cli.md).

Replace the operation with `{"kind":"shutdown"}` for explicit clean shutdown. Project/Task request schemas are generated by `cargo run -p symbiote-protocol --example protocol_schema --locked`; [protocol.md](protocol.md) documents authorization and compatibility. Pretty-printed request files are accepted by the CLI. RPC errors remain structured JSON on stderr and produce the distinct CLI exit status 2.

## Durable boundaries

The daemon holds [the SQLite store](storage.md), not the CLI. Successful mutation receipts follow committed current-state/journal transactions. A lost response can be retried with the same command ID and identical intent; changing the correlation ID does not change intent. Registration reuses its original stored authority timestamp on retries, including after daemon restart. Clients cannot submit actor identities, histories, evidence, or task completion transitions.

Frames have a 64 KiB request bound and absolute two-second read/write deadlines. Responses have a 1 MiB serialization bound; large journal pages return `resource_exhausted` so callers can request fewer events. One request is served per connection. The initial accept loop serializes calls; runtime execution and long operations must not be added to this loop. Client disconnect does not stop the daemon or undo committed metadata. SIGKILL recovery is tested against the actual daemon binary.

## Verification and remaining acceptance

Actual process tests cover CLI attach, clean shutdown, forced daemon kill/restart, replayed registration timestamps, task persistence, journal cursors, dropped responses, concurrent retries, malformed/spoofed requests and cross-Project references. Socket fixtures cover exclusive/stale locks, symlinks, shared directory refusal and bounded frames; slow-reader/writer tests verify absolute deadlines.The authorization gate is proven against a real fake daemon socket rather
than asserted in prose: every dangerous operation without `--yes` exits 3
with an empty accept — the listener sees not one byte — while `--yes` sends
it, a read through `raw` is not gated while `raw` naming `shutdown` is,
read-only commands are never gated, an unreachable daemon is exit 1 rather
than 3, and the `--json` envelope carries the versioned schema for both
outcomes. A unit test also pins that every typed command declares exactly
the operation kind it builds, so the table and the builders cannot silently
drift.

The policy path is proven the same way, on the wire: a policy pre-authorizes
a named kind with no flag and no tty (and through `raw` by the operation it
names), it does not authorize a kind it does not name, an expired policy
sends nothing and names the lapse, a missing or shared or unknown-schema
policy is exit 1 with an empty accept and `policy_invalid` in the envelope
while the same broken policy still lets `health` through, and
`$SYMBIOTE_CLI_POLICY` is honored when no flag is given and overridden when
one is. A separate fake daemon that answers with the daemon's own typed `Err`
pins that a policy authorizes *sending* only — the refusal is still exit 2 —
as does one naming a non-dangerous or unknown kind, which is rejected rather
than treated as a grant. Unit tests pin the load discipline (private mode,
symlink, directory, 64 KiB bound, unknown key, empty list, wildcard,
non-dangerous and unknown kinds, expiry) and the operation table itself
without a daemon at all.

The published schemas are validated against the real binary rather than
asserted: `tests/cli_schema_contract.rs` runs the actual `symbiote` and
validates its stdout envelopes (success, daemon refusal, authorization
required, unusable policy, unreachable daemon) against
`symbiote.cli.v1.schema.json`, validates real policy files against
`symbiote.cli-policy.v1.schema.json`, and requires the schema and the CLI to
classify the same boundary documents the same way. Negative controls tamper
with real envelopes and must fail validation, and a unit test pins each
fixture's `const` value and the policy's authorizable-kind `enum` to the
binary's own constants and operation table. See [cli.md](cli.md).

The noninteractive authorization requirement (#54: "explicit flags/policies") is met by `--yes` for a single invocation and by the policy file for unattended runs; that is a policy, not a general config profile. Linux only: Windows/macOS authentication transports, authenticated WebSocket, the cache/log/runtime directories and user service packaging (this Host resolves its state and configuration directories, which `tests/paths.rs` drives through the real binary), upgrades/drain, workforce execution, process supervision, filesystem permissions, credential vaults, durable budget/outbox effects and GUI attachment remain owned by their canonical issues. Administrative scope beyond the gate is still pending on #54: remote endpoint selection and pairing, shell completion, config profiles and the remaining documented environment variables beyond `SYMBIOTE_CLI_POLICY`, redaction, and goal/child intervention commands. No Host or Preview permission claim is inferred from a same-UID local connection. The separate fixed-workload lifecycle spike is not silently promoted into production process supervision. Broad #180/#181/#43 acceptance remains open.
