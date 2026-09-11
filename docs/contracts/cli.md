# `symbiote` CLI automation contract — #54

The `symbiote` binary is the administrative client for a running `symbioted`; its
command surface, authorization gate and exit codes are documented in
[host.md](host.md). This page publishes the two machine-readable documents that
page describes in prose, as versioned JSON Schema fixtures, and states exactly
what the repository proves about them:

- [`schemas/symbiote.cli.v1.schema.json`](schemas/symbiote.cli.v1.schema.json) —
  one `--json` envelope.
- [`schemas/symbiote.cli-policy.v1.schema.json`](schemas/symbiote.cli-policy.v1.schema.json)
  — an authorization policy file.

Both are JSON Schema **draft 2020-12**. Automation keys on the `schema` string
inside the document, never on the presence of individual fields, so a future
revision is a new `$id` rather than a silent reinterpretation of this one.

The binary emits both documents: `symbiote schema` prints a JSON object keyed
by schema identity (`symbiote.cli/v1`, `symbiote.cli-policy/v1`). It is a local
command — no daemon, no `--state-dir`, no authorization — so any installed
binary can hand out the contract it implements. `symbiote schema --write DIR`
regenerates the committed fixture files instead of printing them, so the
fixtures are **generated artifacts**: a contract change is made once in the
binary and then regenerated, never edited into a fixture by hand.

Flags are declared per command, and a flag a command cannot honor is a usage
error that names it, never a silent no-op. The daemon commands honor the five
daemon-facing flags (`--state-dir`, `--command-id`, `--policy`, `--json`,
`--yes`), because each of them is answered by a request over the socket; the
local `schema` command honors only `--write`, and the local `help` command
honors nothing. So `symbiote --write DIR health`, `symbiote --json help`,
`symbiote --state-dir DIR schema` and `symbiote schema --yes` each exit 1
printing nothing, while `symbiote --state-dir DIR shutdown --yes` and
`symbiote --json health` are unaffected.

The contract itself lives in `symbiote_host::cli_schema` (`crates/symbiote-host/src/cli_schema.rs`):
the schema identities, the `OPERATION_RISKS` table that fixes the policy
schema's `enum`, the document builders, the published-document table, and the
fixture directory. The `symbiote` binary is the interface — it renders that
contract, classifies operations with it, and decides authorization — so no
second owner can hold a copy that drifts.

## Exit codes

Every invocation ends in one of four codes. They are distinct so a script can
tell a refusal from a transport failure from a missing authorization without
parsing prose.

| Code | Meaning | Envelope |
| --- | --- | --- |
| `0` | Success. | `ok: true`, `result` present |
| `1` | Usage or connection failure — a bad flag, a missing `--state-dir`, an unusable policy, or an unreachable daemon. | `--json` pairs an unusable policy and an unreachable daemon with `ok: false`; a usage failure prints no envelope at all. |
| `2` | The daemon refused the command (its typed `Err`). | `ok: false`, `error.code` is the daemon's |
| `3` | Authorization required: nothing was sent. | `ok: false`, `error.code` is `authorization_required` |

## The `--json` envelope

`--json` prints **one** envelope per invocation on stdout instead of the
daemon's pretty-printed body. The default output is unchanged, so existing
scripts that read pretty JSON keep working. The envelope is a flat object: four
keys always present, plus exactly one of `result` and `error`.

The local `schema` and `help` commands do not accept `--json`; `schema`
refuses it rather than reinterpreting it: `symbiote schema` already prints
machine-readable JSON, and that JSON is not this envelope — the documents have
no `result.kind`, so wrapping them would either violate the published envelope
schema or invent a protocol result body. `symbiote schema --json` and
`symbiote --json help` are usage errors (exit 1) that print nothing, per the
flag rule above.

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | `const "symbiote.cli/v1"` | Schema identity. |
| `command` | non-empty string | The command name as invoked, e.g. `health`, or `raw`. |
| `command_id` | non-empty string | The idempotency key this run used: the minted id, or a caller-supplied `--command-id`. |
| `ok` | boolean | True exactly when `result` is present. |
| `result` | object | Present on success: the daemon's own body, **unmodified**. |
| `error` | `{code, message}` | Present on failure: the daemon's typed error, or the CLI's own code. |

Exactly one of `result` and `error` is present, and `ok` agrees with which: the
schema encodes that as a `oneOf`, so a document carrying both, or neither, is
invalid even though each field is individually well-formed.

`result` is the serialized `symbiote_protocol::ResponseBody`, which is
internally tagged on `kind` — so `result` is always an object whose `kind`
names the **answer**, not the command. `health` is answered with a `hello`
body, for example, and `shutdown` with `{"kind":"shutdown"}`. The schema
requires `kind` and leaves the rest of the body to
[protocol.md](protocol.md); it does not restate every variant.

`error.code` is a non-empty string. When the daemon answered, it is the
daemon's own code. When nothing reached the socket, it is one of the CLI's
three:

- `authorization_required` — a dangerous operation without `--yes`, a policy
  grant, or an accepted prompt. Exit 3.
- `policy_invalid` — a configured policy that could not be honored: missing,
  unreadable, insecure, oversized, the wrong schema, or naming a kind a policy
  may not name. Exit 1. The CLI never downgrades this to "no policy".
- `unreachable` — the daemon could not be reached, or answered with an
  unparseable frame. Exit 1.

```json
{"schema":"symbiote.cli/v1","command":"shutdown","command_id":"cli-shutdown-1757556000000-1234-0","ok":true,"result":{"kind":"shutdown"}}
{"schema":"symbiote.cli/v1","command":"shutdown","command_id":"cli-shutdown-1757556000000-1234-0","ok":false,"error":{"code":"authorization_required","message":"noninteractive runs require --yes or a policy naming \"shutdown\""}}
{"schema":"symbiote.cli/v1","command":"shutdown","command_id":"cli-shutdown-1757556000000-1234-0","ok":false,"error":{"code":"policy_invalid","message":"cannot use the policy /etc/symbiote/policy.json: the policy is not a valid symbiote.cli-policy/v1 document"}}
```

## The policy document

A dangerous operation is sent without `--yes` only when a policy
pre-authorizes its operation kind. `--policy FILE`, or `$SYMBIOTE_CLI_POLICY`
when the flag is absent, names a document with three keys:

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | `const "symbiote.cli-policy/v1"` | Schema identity. Any other value is refused. |
| `authorize` | array, ≥ 1, of dangerous kinds | The kinds pre-authorized. Kinds, not command names. |
| `expires_at` | integer, `0 ..= u64::MAX` | Optional unix-millisecond bound. |

```json
{"schema":"symbiote.cli-policy/v1","authorize":["shutdown"],"expires_at":4102444800000}
```

`authorize` may name **only** the kinds this CLI classifies as dangerous, and
the schema's `enum` is exactly that list — `decide_elevation`,
`record_resource_consent`, `run_started_dispatch`, `shutdown`,
`start_prepared_task`. A read or a mutation is already ungated, so listing one
would be a no-op an operator could mistake for coverage; an unknown kind cannot
be classified; and a wildcard is deliberately not a syntax. Because the grants
are a set, repeating a kind is accepted and collapses rather than being an
error. The kinds are operations, so a policy listing `shutdown` authorizes both
the typed `shutdown` command and `raw shutdown.json`, and neither when it lists
something else.

The document grants authority, so it is held to the same private-file
discipline the Host applies to its own operator config, enforced at load:
a regular file opened with `O_NOFOLLOW` (a symlink is refused, not followed),
owned by this user, with no group/other permission bits, bounded at 64 KiB.
Those are filesystem properties, not schema properties — a schema validator
alone cannot check them, and passing the schema is necessary but not
sufficient for a policy to be honored.

## How these fixtures are kept honest

The fixtures are the published contract and the repository keeps them from
drifting from the binary in three places, because a schema that has quietly
drifted is worse than none:

- **Against the binary's own output** (`tests/cli_schema_contract.rs`):
  `symbiote schema` is run and each emitted document must equal its committed
  fixture **exactly**, so a fixture edited on its own — or a schema built
  differently in code — fails the build. A second test runs `symbiote schema
  --write` into a temporary directory and requires the files it writes to be
  byte-for-byte the committed ones, so regenerating a clean tree is a no-op.
- **Against the code** (`src/cli_schema.rs` unit tests, in the contract
  module): the builders' canonical bytes must equal the committed fixtures,
  the envelope fixture's `const` must equal `CLI_SCHEMA`, the policy fixture's
  `const` must equal `POLICY_SCHEMA`, and the policy fixture's `enum` must be
  exactly the kinds the operation-risk table marks dangerous. This pins the
  builders in-process, so drift fails `cargo test --lib` without a binary or a
  daemon; a new dangerous kind that is not published fails the build.
- **Against real output** (`tests/cli_schema_contract.rs`): the real
  `symbiote` binary is run against a fake daemon socket and its actual stdout
  envelopes are validated against the envelope fixture — success, daemon
  refusal, authorization required, unusable policy, and unreachable daemon. The
  real policy files are validated against the policy fixture, and the schema
  and the CLI are required to **agree** on a set of boundary documents: a
  document one accepts and the other rejects is a failure. Negative controls
  tamper with real envelopes and require validation to fail, so a test cannot
  pass by ignoring a keyword.

The in-repo checker is deliberately small and **closed**: it supports exactly
the keywords the two fixtures use — `$ref` (local `#/$defs/…` only), `type`,
`const`, `enum`, `required`, `properties`, `additionalProperties`, `items`,
`minItems`, `minLength`, `minimum`, `maximum`, `oneOf`, `not`, plus the
annotations `$schema`, `$id`, `title`, `description` and the `$defs`
container — and refuses any fixture that
uses a keyword outside that set. That is what makes the validation meaningful
rather than decorative: a constraint the checker did not implement would be a
claim the test silently ignored, so adding one to a fixture without teaching
the checker fails the build first. The fixtures use no keyword that requires a
regular expression engine. Any conforming draft 2020-12 validator can consume
them too; the subset is a property of this repository's test, not of the
schemas.

Two boundaries are named rather than hidden. JSON Schema's `integer` is defined
on the mathematical value, so a conforming validator accepts
`"expires_at": 1.0`; the loader reads a `u64` and refuses a non-integer lexical
form, so the CLI is *stricter* there than the schema — a strictness no schema
can express. And the in-repo checker compares `minimum`/`maximum` exactly for
representable integers but falls back to `f64` above that, so it cannot
distinguish a value just past `u64::MAX` from `u64::MAX` itself — the fixture's
`maximum` is still correct, and an arbitrary-precision validator enforces it.
The agreement test covers the boundaries both sides can represent.

## Verification and remaining acceptance

`cargo test -p symbiote-host` runs both halves. The contract module's unit
tests compare the builders' canonical bytes to the fixture files under
`docs/contracts/schemas/` and pin them to its own constants and risk table. The
integration test runs the actual binary and
validates its stdout through the bounded checker, including the boundary
documents the schema and the CLI must classify identically. CI additionally
regenerates the fixtures with `symbiote schema --write` into a temporary
directory and fails on any `diff`, so a hand-edited fixture is rejected even
before the tests run. The default, non-`--json` output is the daemon's body
alone and is deliberately not covered by an envelope schema.

Published fixtures are not a compatibility policy: this page documents v1, and
a v2 requires its own `$id`, a compatibility note and migration tests before
the wire assumptions change. Administrative scope beyond this contract is
still pending on #54: remote endpoint selection and pairing, shell completion,
config profiles, redaction, and goal/child intervention commands. No Host or
Preview permission claim is inferred from a same-UID local connection.
