# Runtime discovery foundation (#186)

`symbiote-runtime-discovery` provides version 1 inventory contracts and an actual
read-only Codex App Server probe. It grants no execution, installation, login,
credential or billing authority. #186 remains open for its full acceptance.

## Installation observation slice

`installation::ExecutableInstallation::inspect` is the side-effect-free
metadata pass. The Host supplies an absolute candidate and a trusted root; the
pass canonicalizes the candidate, checks that neither the candidate nor its
parents escape that root, requires a regular non-writable executable, and keeps
the requested and resolved paths. It never starts the candidate. Its channel is
the Host-declared `InstallationChannel`, not a name-based guess.

A real adapter may then pass separately obtained `ProtocolEvidence` to
`confirm_protocol`. That is the only path by which version, interface, SDK, or
`ConfigRoot` observations become known. The Codex proof binds the inspected
`/usr/bin/codex` to its sandboxed App Server response and isolated config root;
a model listing still does not prove model usability. This slice has no
authenticated update check, install/update consent, logout, entitlement, or
credential flow, so `UpdateAvailability::Unknown` is deliberate rather than a
successful update result. The observation is Host-local and its exact paths are
not a public account or secret export.

The named refusals `OutsideTrustedRoot`, `MutableExecutable`,
`NotExecutable`, `MissingExecutable`, `InvalidProtocolEvidence`, `Stale`, and
`ResourceLimit` are part of the boundary. Deserialization repeats the same
validation; a serialized caller cannot turn an unknown update fact into a claim,
and SDK remains unknown unless protocol evidence explicitly supplies it. Multiple
config roots are bounded and deduplicated by opaque identity and path, while
account names, tokens, and email addresses remain outside the contract.

## Named profile resolution

`profiles::resolve_profiles` turns adapter-declared `ProfileSpec` values into
bounded `ProfileObservation` values using an explicitly supplied environment
map and Home path. An adapter chooses the variable name (`CODEX_HOME` is one
example); the core does not branch on a harness name. An absolute environment
override wins, otherwise the declared relative default is resolved beneath
Home. The resolver never reads or mutates the process environment, creates a
directory, authenticates, or selects an account for execution.

Each observation keeps the display label, opaque config identity, optional
Host-generated `account_ref`, exact Host-local path, and whether the path came
from `EnvironmentOverride` or `ProfileDefault`. Duplicate profile IDs, config
identities, environment variables, default declarations, or resolved paths are
refused; the resolver names these `DuplicateProfile`, `DuplicateConfig`,
`DuplicateEnvironment`, and `DuplicatePath` refusals. Invalid relative/absolute paths, malformed account references, stale
observations, and the bounded profile count have named refusals. Exact paths are
redacted from `Debug`; profile resolution is not an account or credential
export.

The Codex proof now resolves its disposable profile through this contract before
the App Server probe. This demonstrates alternate config-root selection without
changing `HOME`; it does not prove that an account is authenticated, that two
accounts have separate quotas, or that a profile may activate a worker.

## Real Codex adapter record

`codex_adapter::bind_report` is the composition boundary between the real
read-only App Server adapter and the versioned `Inventory`. It accepts the
`RuntimeInstanceIntent`, resolved `ProfileObservation`, confirmed
`ExecutableInstallation`, and completed `CodexProbeReport` together, then
cross-checks their profile, config identity, exact config root, Host, runtime
kind, adapter, installation, interface, version, and freshness before publishing
one `DiscoveryRecord`. The probe producer also captures the directly owned
process identity and validates the App Server's absolute `codexHome`; both are
carried as non-forgeable report evidence and checked against the process and
profile the adapter publishes. A report cannot be paired with another process,
named instance, executable, root, Host, or adapter and still validate.

Reachable health and the methods the handshake actually called are recorded.
Required and not-required authentication remain explicit. An account object
reported by Codex does not establish credential validity, entitlement, account
identity, or subscription authorization, so the authentication fact remains
unknown and no account payload is copied. Model listings retain upstream
identity, leave canonical mapping and absent context limits unknown, and do not
make a model usable. Duplicate upstream model identities, stale profile windows,
unconfirmed or mismatched interfaces, and cross-instance pairings receive named
refusals: `ProcessMismatch`, `ProfileMismatch`, `IdentityMismatch`,
`InterfaceMismatch`, `DuplicateModel`, `UnconfirmedProtocol`,
`InvalidObservation`, and `Stale`. The real Codex proof
now emits this validated inventory record after its sandboxed probe; the
inventory is an observation, not a durable Host endpoint or activation permit.

Desired instance identity is separate from observations: Host, runtime kind,
adapter, installation, profile and symbolic configuration identity must match.
Qualification checks freshness, pinned interface/version and explicit required
facts. Missing context limits, isolation or methods remain unknown. A model
listing is not proof the account can use that model. External model strings are
distinct from canonical Model IDs. Discovery cannot authorize worker activation.
Caller-created observations require trusted Host provenance; serialized provenance
labels are not signatures. The inventory is bounded and currently in memory.

## Durable Host inventory read surface

An inventory was, until this slice, in memory: a discovery run produced records
and the process that ran it dropped them. A Host now serves one, and the wire
surface is a single owner-only read, `get_runtime_inventory` (protocol v1.25),
answered from a private file in the Host's state directory named
`runtime-inventory.json`. The file is this Host's own record of this machine; a
record that names another Host is not this Host's document and is refused.

The read applies the discipline the Host already applies to its own identity and
policy files: it opens the published name with `O_NOFOLLOW` and classifies the
*opened handle* — a regular file, owned by this user, mode 0600, with no second
name for it — so a path swapped for a symlink between a check and a read cannot
slip a different document past the gate. The document is then re-validated whole
by this crate's own parser on every read. Nothing is cached: the file is the
durable record, so a read after a republish answers from the new document and
survives a restart. The published bound is 512 KiB, applied to the file before it
is parsed and well inside the protocol's 1 MiB response bound, so a document
this Host will serve always fits its own response envelope; an inventory larger
than that is refused by name rather than discovered at the transport.

Refusals are a closed vocabulary and the read is never partially served:
`no_published_inventory`, `unsafe_inventory_file`, `unreadable_inventory`,
`unparseable_inventory`, `oversized_inventory` and `foreign_host_inventory`. The
names do not distinguish which private-file check failed, because a reader that
could tell "wrong mode" from "foreign-owned" would be an oracle for probing this
Host's state directory, and none of them carries a path or a value the document
said.

A record past its own `expires_at` is served, not filtered: expiry is the
reader's call, and dropping records on the way out would make the document
disagree with what this Host actually holds. Serving an inventory decides
nothing — qualification, diagnostics and activation remain separate, and the
read grants no activation, credential, installation or billing authority.

There is no client write path. The document is installed by an operator on the
machine with `symbiote publish-runtime-inventory DOCUMENT --state-dir DIR`
(see [cli.md](cli.md)), which re-validates it whole, requires every record to
name that state directory's own Host identity, and installs it atomically at
mode 0600 so a reader sees the old document or the new one. A refused document is
named and leaves the Host serving what it already held. What a discovery run
produces and an operator publishes is still two steps with a person in the
middle: this slice makes the inventory durable and readable, not discovered on a
schedule.

## Deterministic diagnostics

`diagnostics::diagnose` maps only the closed source errors that correspond to
#186's required operator outcomes. `DiagnosticInput` accepts the existing typed
installation, profile, qualification and Codex errors; it cannot carry an
arbitrary message, path, account or credential. The versioned
`RuntimeDiagnostic` contains only `DiagnosticCode`, its non-authorizing
`DiagnosticAction`, and schema version. Deserialization rejects unknown fields,
unsupported versions and a code/action pair that is not the one prescribed by
the contract. The published schema independently fixes the accepted version and
enumerates the same five code/action pairs, so schema-only consumers cannot
accept a broader diagnostic than the runtime parser.

The five deterministic mappings are: `MissingExecutable` to
`MissingBinary` / `InstallBinaryWithConsent`; `CodexDiscoveryError::UnsupportedVersion`
or qualification `VersionMismatch` to `UnsupportedVersion` /
`UpgradeWithConsent`; `AuthenticationExpired` to
`ExpiredAuthentication` / `Reauthenticate`; `RateLimited` to `RateLimited` /
`RetryLater`; and invalid, duplicate or non-absolute profile declarations to
`IncompatibleConfiguration` / `CorrectConfiguration`. Install, upgrade and login
words are recommendations for explicit user-visible flows. This module performs
none of them and holds no consent, credential or activation authority. Other
refusals remain their original typed error rather than being broadened into a
possibly unsafe guess. The schema example publishes the diagnostic contract.

## Pinned Codex interface

The tested release is `codex-cli 0.118.0`. The probe uses versionless JSONL RPC:
`initialize`, `initialized`, `account/read` with `refreshToken: false`, then bounded
`model/list` pagination. It never sends login, logout, thread or turn requests.
Frames, notifications, pages, IDs and the overall deadline are bounded. Raw
account identities, error payloads and user agents are not retained in reports.
An account type is reported metadata, not a verified credential or billing route.

The [official App Server documentation](https://learn.chatgpt.com/docs/app-server)
describes the handshake and methods. Version-specific shapes were checked against
the installed executable's `app-server generate-json-schema` output, without
experimental methods. Actual initialization reports `symbiote/0.118.0` for our
client name, despite clientInfo.version being `0.1.0`. The live probe caught the
initial incorrect `codex_cli_rs` prefix assumption. Context limits are absent from
this pinned model schema and remain unknown.

## Actual Linux proof

Build `symbiote-sandbox-launch`, then run the `codex_discover` example with its
absolute path, an empty worktree under a private 0700 parent, and a separate
protected Host directory. The optional
`--inventory-document=PATH` names a file to write the inventory document this
run produced, so the document an operator publishes into a Host is the one a real
discovery run built; the file is the producer's output and carries no Host
identity of its own, so the Host still installs it only after checking every
record against its own. It is spelled as an option rather than a fourth
positional because the protected-directory list is variadic: a trailing path
would be read as one more protected directory and refused. The sandbox runs `/usr/bin/codex` with a disposable
`/home/agent` HOME, an empty environment, read-only project and isolated network.
Initialization must report `/home/agent/.codex`. No real account directory is
mounted. The runner checks cancellation errors; descendant cleanup remains
`Unknown` under the shared transport contract.

The local installed binary returned version `0.118.0`, authentication `required`
and six listed models with model usability unknown. No model turn was started.
CI repeats this with the exact Linux x64 npm artifact and its SHA-512 integrity
before installing the binary on the disposable runner only. CI does not update
the user's Codex installation. Unit fixtures prove rejection contracts; the real
binary proof establishes this narrow offline protocol compatibility only.

Pending: a Host-side probe that discovers on a schedule rather than an operator
publishing a document, configured profile/account binding, native-provider
discovery, executable provenance beyond the trusted `/usr`
premise, user-visible install/update/logout/default-change consent flows,
authentication/quota/entitlement health integration, account isolation acceptance,
authenticated online model calls and coding tasks. Deterministic diagnostics do
not perform their recommended actions. No credentials, spending, runtime
activation or first-release completion are implied by this foundation.
