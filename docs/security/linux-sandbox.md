# Linux execution sandbox

Canonical owner: #218. This implementation slice consumes the runtime transport
and exact-resource consent foundations of #184/#185/#191. It does not activate
native or Codex workers through the Host protocol. Those integrations and the
full #218 acceptance criteria remain pending.

## Boundary

The intended boundary is an untrusted child process inside Linux namespaces.
The Host, its current policy and persisted consent records, the worktree
provisioner, kernel, bubblewrap installation and read-only system tool tree are
trusted. A malicious process already running outside the sandbox under the
Host's Unix account is outside this boundary: it can reach same-user files and
the existing bootstrap Host socket. Never give a worker an unsandboxed fallback.

The launcher supports read-only and worktree-write execution with network
isolation. Other profiles are not implemented. Host integrations must obtain
current persisted consent and current policy, resolve the canonical Root binding
and provide all protected Host state and credential paths before invoking it.
Supplying Rust structs is not authentication. A command fingerprint identifies
the launch description; it does not certify executable bytes, transitive
dependencies or a project's content. Resource loading still needs its own
content-bound consent and check/use guarantees.

The internal entry point is `symbiote_sandbox::launch(LaunchRequest)`. Its
non-serialized request carries the trusted consent, observed command snapshot,
current access policy, Host identity, time, Root ID, reserved worktree, protected
directories, profile, program arguments and transport bounds. Only explicit
`/usr/bin` commands are admitted. Both profiles require `ReadRoot` and
`ExecuteProcess`; worktree-write also requires `MutateStream`. The command
fingerprint includes the fixed invocation version, Root, absolute worktree,
profile, program and arguments. Changing them requires matching consent.

The returned process supports bounded JSONL send/receive, exit inspection and
cancellation. Launch setup must succeed before a process handle is returned;
that does not imply that the target command succeeds. Its eventual exit and
verification evidence remain separate completion gates.

The system tool tree is readable inside the sandbox. Do not store credentials
there. No ambient environment, home directory, Host socket, canonical database,
credential directory or unrelated repository should be mounted. The bound
worktree needs a private parent controlled by the trusted provisioner, which
must not expose Host IPC endpoints, devices or hardlinks to protected files.
Namespace isolation does not make a maliciously provisioned mount safe.

A live adversarial probe demonstrated why: a pathname Unix socket inside a
mounted worktree reached a Host listener despite network namespace isolation,
and a writable hardlink changed a file outside that mount. See the
[alias probe evidence](../proofs/linux-sandbox-aliases.md). A filesystem preflight
must reject these entries before launch. Such a check relies on the trusted
provisioner reserving the source tree throughout execution: only the sandboxed
process may perform permitted writes. It cannot protect against a concurrent
unsandboxed process inserting or replacing entries, even after the mount exists.

This initial preflight conservatively rejects all symlinks, multiple hardlinks
and non-file/non-directory entries. It limits traversal to 100,000 entries and
64 levels. Trees with unsupported entries require a future safe staging design;
do not silently remove the checks to accommodate a project. Source tree owners
and modes are checked, and the immediate parent must be owned and mode 0700.

Inherited file descriptors are another independent boundary. Bubblewrap was
observed preserving extra descriptors supplied by its parent. The trusted
single-threaded launcher helper must close all descriptors above stderr before
executing bubblewrap; clearing environment variables and hiding paths do not
revoke an already-open descriptor. Host stdin/stdout/stderr use the bounded
transport pipes. The helper executable itself is trusted installation content,
not a project-supplied executable or a resource selected by a prompt.

## Verification and remaining gates

Tests must execute the real installed bubblewrap. An unavailable or rejected
namespace setup is a failure, not a skipped security test. CI builds bubblewrap
0.12.0 from upstream commit `2a76602a8c71f36c1527cf9fc3417d9149822e0c` on Ubuntu
22.04 and records its version without disabling host security policy. The local
starting environment also provides bubblewrap 0.12.0. The upstream source is
[pinned here](https://github.com/containers/bubblewrap/commit/2a76602a8c71f36c1527cf9fc3417d9149822e0c).

The initial Ubuntu 24.04 runner with distribution bubblewrap 0.9.0 failed actual
namespace setup: `bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted`.
[The failed setup run](https://github.com/RepairYourTech/SymbioteIDE/actions/runs/34194047726)
remains evidence of unsupported configuration, not a passing platform result.
No AppArmor/sysctl policy was disabled, and no invocation flags were relaxed.
Ubuntu 24.04 compatibility needs a separate verified installation configuration.

Run the focused verification with:

```sh
cargo test -p symbiote-sandbox --locked
```

The tests require Linux, Python 3, bubblewrap and working unprivileged namespace
creation. The launcher uses the merged `/usr` system layout and a separately
built `symbiote-sandbox-launch` helper. Packaging must install that helper in a
trusted location and pass its absolute path from Host configuration. Never
search a project's PATH for it. Unsupported layouts or namespace policy must
produce setup failure; portability needs actual platform evidence.

The process transport retains bounded JSONL framing and diagnostics. Cancellation
evidence must distinguish the launcher exit from descendant termination; process
group signalling alone cannot prove that a setsid child is gone. Owner-death
tests must cover that distinction before making a namespace cleanup claim.

Local verification on 2026-09-08 passed all ten sandbox test entries (including
two subprocess fixture entry points), the workspace suite, strict workspace
Clippy and formatting. Tests exercise consent changes/expiry/revocation and
permission denial, aliases, command/path bounds, inaccessible Host files and
network, inherited fixture descriptors, permitted writes, cancellation and
owner death. The last two observe a positive heartbeat stop; the public
descendant-cleanup result deliberately remains `Unknown`. They do not certify
every possible descendant or outstanding privileged side effect.

Setup failures retain at most eight diagnostic lines of 1,024 UTF-8 bytes each,
with truncation/incomplete-drain status. Debug and Display redact those lines;
trusted callers must explicitly request them for troubleshooting and apply their
own disclosure policy. This preserves the reason for a rejected namespace setup
without automatically logging potentially sensitive child text.

The `sandbox_probe` example was also executed with a private temporary fixture.
It requires the exact report below and a successful target exit; it uses fixture
consent and performs no authenticated Host approval or paid inference:

```json
{"home":"/home/agent","sandbox":true,"write_denied":true}
```

The local Rust 1.85 installation reports a missing toolchain manifest. The CI
matrix installs its own toolchains and is the minimum-version integration gate;
the local failure is not a passing MSRV result.

Pending acceptance includes production worktree provisioning, restricted worker
protocol and authenticated launch integration, durable decision/effect journals,
resource limits and disk quotas, network mediation, other sandbox profiles,
active consent revocation, MCP/browser/hook mediation, real native/Codex packs,
Windows/macOS enforcement and the complete first-release demonstration. This
slice introduces no durable storage migration or user credential changes.
