# Local durable Host — #180 / #181 / #43

The workspace now builds `symbioted` and its `symbiote` CLI client. This is the first persistent metadata service, not an agent runtime. It registers canonical Projects, Roots and Roles together, creates ready Tasks with initial Change Streams, returns records and paginates a durable journal. It does not execute commands, call models or claim task completion. Dispatch activation (`run_started_dispatch`) provisions the task's Change Stream worktree from the Root's recorded host placement — reservation verification, base-commit validation against the stream's recorded base, and materialization — before any transport would be built; with the production configuration (no reservation base, no transports) it refuses with typed errors and nothing executes.

## Run

```sh
cargo build --workspace --locked
target/debug/symbioted --state-dir /absolute/private/state-directory
```

The parent directory must exist. A new final directory is created mode 0700; an existing shared, foreign-owned or symlink directory is refused without changing its permissions. The local socket is mode 0600. Each connection uses Linux SO_PEERCRED to establish same-UID ownership; this is an explicit local-owner policy, not remote or Preview authentication. No TCP listener is opened. An exclusive lock prevents a second daemon replacing the live socket; restart replaces only a stale owned socket after acquiring that lock.

Send a request from another terminal:

```sh
target/debug/symbiote --state-dir /absolute/private/state-directory health
```

## Administrative command surface

`symbiote --state-dir DIR <command> [args...]` maps onto typed protocol
operations — `hello`, `health`, `host-pulse`, `shutdown`, `get-project`,
`create-task`, `get-task`, `read-journal`, `prepare-dispatch`,
`get-dispatch-preparation`, `start-prepared-task`, `run-started-dispatch`,
`request-task-completion`, `scheduling-projection`, `get-team`,
`get-task-origin`, and `raw <operation.json>` for anything the typed
commands do not cover (the daemon still validates and authorizes every
field). Responses are the daemon's own JSON, pretty-printed. Exit codes
are distinct: 0 success, 1 usage/connection failure, 2 a daemon-refused
command (`Err` response), 3 authorization required — scripts branch on
refusal vs transport failure vs a missing authorization.
`help` lists the command table, marking dangerous commands and naming the
dangerous kinds a policy may pre-authorize. Identity arguments are passed as
plain strings and typed on the wire; no caller-supplied actor identities
exist. The interactive/headless coding-agent experience is #467's surface and
shares this client plumbing; this binary carries no model loop.

### Dangerous operations require explicit authorization

Authorization follows the **operation**, not the command name. One table
classifies operation kinds, and both the typed commands and `raw` are
decided by it — so `raw shutdown.json` cannot bypass the gate on
`shutdown`, and a read through `raw` is not over-gated. Dangerous kinds are
the ones that start or end execution or grant capability:
`shutdown`, `start_prepared_task`, `run_started_dispatch`,
`decide_elevation` (the grant; `revoke_elevation` narrows and is not
gated) and `record_resource_consent`. A kind the table does not know
cannot be proven safe, so it fails closed as dangerous. Reads are free;
everything else is a mutation the daemon still authorizes — filing
evidence (`create-task`, `prepare-dispatch`, `request-task-completion`,
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
invocation on stdout instead of pretty output: `schema` is the versioned
`symbiote.cli/v1`, alongside `command`, `command_id`, `ok`, and either
`result` (the daemon's own body, unmodified) or `error.code` — the daemon's
error code, or the CLI's own `authorization_required` (nothing was sent),
`policy_invalid` (the configured policy could not be honored; nothing was
sent) and `unreachable`. The default output is unchanged, so existing scripts
that read pretty JSON keep working. Minted command IDs carry millisecond,
pid and an in-process sequence, so two invocations cannot share an
idempotency key and have one mistaken for a replay of the other.

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

The noninteractive authorization requirement (#54: "explicit flags/policies") is met by `--yes` for a single invocation and by the policy file for unattended runs; that is a policy, not a general config profile. Linux only: Windows/macOS authentication transports, authenticated WebSocket, XDG defaults, user service packaging, upgrades/drain, workforce execution, process supervision, filesystem permissions, credential vaults, durable budget/outbox effects and GUI attachment remain owned by their canonical issues. Administrative scope beyond the gate is still pending on #54: remote endpoint selection and pairing, shell completion, config profiles and the remaining documented environment variables beyond `SYMBIOTE_CLI_POLICY`, redaction, published JSON Schema fixtures for the `--json` envelope (including the policy document), and goal/child intervention commands. No Host or Preview permission claim is inferred from a same-UID local connection. The separate fixed-workload lifecycle spike is not silently promoted into production process supervision. Broad #180/#181/#43 acceptance remains open.
