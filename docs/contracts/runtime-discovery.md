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
config roots are bounded and deduplicated by opaque identity and
path, while account names, tokens, and email addresses remain outside the
contract.

Desired instance identity is separate from observations: Host, runtime kind,
adapter, installation, profile and symbolic configuration identity must match.
Qualification checks freshness, pinned interface/version and explicit required
facts. Missing context limits, isolation or methods remain unknown. A model
listing is not proof the account can use that model. External model strings are
distinct from canonical Model IDs. Discovery cannot authorize worker activation.
Caller-created observations require trusted Host provenance; serialized provenance
labels are not signatures. The inventory is bounded and currently in memory.

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
protected Host directory. The sandbox runs `/usr/bin/codex` with a disposable
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

Pending: durable inventory/Host endpoints, discovery of configured profiles,
native-provider discovery, executable provenance beyond the trusted `/usr`
premise, explicit install/update consent, authentication/quota/entitlement health,
account isolation acceptance, authenticated online model calls and coding tasks.
No credentials, spending, runtime activation or first-release completion are
implied by this foundation.
