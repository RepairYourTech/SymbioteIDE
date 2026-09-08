# Local structured transport

Owner: [#185](https://github.com/RepairYourTech/SymbioteIDE/issues/185), consuming the foundational Host lifecycle and runtime SDK contracts from #180/#184. This is a local subprocess substrate. It is not a complete harness pack, sandbox, or production agent activation path.

`symbiote-runtime-transport` provides newline-delimited JSON process I/O with explicit executable, arguments, working directory and environment. The parent environment is not implicitly inherited. Callers must supply an authorized launch configuration; the library does not turn a file path or executable presence into trust or permission. Never put credentials in command-line arguments.

Stdout carries structured frames; stderr is a separate diagnostic channel. Both must remain bounded even for a stalled consumer or hostile child. Transport decoding does not make an upstream tool request authorized, a worker completion request successful, or a reported usage/capability fact verified. The canonical Host remains responsible for those decisions.

`TransportLimits` defaults to 64 KiB per frame, 32 queued frames, 32 diagnostics and 4096 bytes per diagnostic. Invalid JSON, duplicate object keys (including nested keys), unterminated frames, oversized stdout and a full frame queue produce a sticky failure. A partial stdin write also faults the connection so later frames cannot append to an incomplete message. A receive timeout is nonfatal. Stderr retains a bounded recent window, exposes truncation and dropped-entry counts, and redacts text from `Debug`; explicitly reading its text requires the caller's privacy policy. Final decoded diagnostic text remains subject to the byte limit even when invalid UTF-8 requires replacement characters.

The Unix implementation starts a new process group and uses nonblocking pipes. `cancel` returns a `CancelReport` separating root exit from the group signal status. Descendant cleanup remains `Unknown`: a worker can escape a group, and once `try_wait` has reaped the leader, safely signalling that original numeric group is no longer possible because its ID may be reused. The library skips that unsafe signal and reports `LeaderAlreadyReaped`. `Drop` attempts bounded cleanup but cannot certify it. This is why the Host does not yet activate untrusted workers through this library. Tests prove cancellation of the controlled, unescaped fixture group while its leader is still owned; they do not prove complete containment. Linux is the tested platform; macOS and Windows evidence is pending.

## RPC correlation

`rpc::RpcSession` is an opt-in, strict subset of [JSON-RPC 2.0](https://www.jsonrpc.org/specification). It creates monotonically increasing string request IDs, bounds the number of outstanding requests and correlates responses independently of arrival order. A response consumes an ID only after envelope validation. Unknown or duplicate response IDs fail. Notifications have no response guarantee.

This subset requires the explicit `jsonrpc: "2.0"` version field, object/array parameters, string response IDs, a single response result or error, and no unknown envelope fields. Batch messages and peer-initiated requests are explicitly unsupported. It does not claim compatibility with versionless app-server envelopes, Codex, or ACP. Those need independently tested pack-specific codecs and capability negotiation; no permissive dialect guessing occurs.

A request is tracked before the caller writes it. An interrupted write leaves its delivery and effects unknown. Consuming a disconnected `RpcSession` returns unresolved IDs for the owning adapter to reconcile; notifications are not tracked. This is not durable replay or permission to retry. The owner must retire the old connection before creating a new generation. Raw response/notification payloads may include source and prompts and deliberately do not implement `Debug`. RPC correlation accepts already decoded `Value` objects; raw frame size, nesting and duplicate-key admission belong to the transport decoder.

## Verification and remaining acceptance

Run `cargo test -p symbiote-runtime-transport` for the deterministic local fixture suite and `cargo test --workspace --locked` for integration with the existing contracts. These tests do not perform inference, consume Codex authentication, or require credentials.

`cargo run -p symbiote-runtime-transport --example jsonl_probe` performs a real subprocess round trip through `/bin/cat` and cancels/reaps that fixture. Its output reports transport proof only; it is not the first-release coding task demonstration.

Independent review reproduced two boundary failures and drove fixes: UTF-8 replacement could expand a diagnostic beyond its configured limit, and a root exit alone could obscure surviving descendants after leader reaping. Regression fixtures cover the decoded byte bound and explicit cancellation uncertainty. Separate RPC review tightened the real-process reordered-response fixture to assert both returned IDs and results.

The transport does not yet implement the SDK `AgentRuntimeAdapter` or the Host activation journal. Complete process-tree containment, Host-crash cleanup, durable reconnect/effect reconciliation, heartbeats, remote Hosts, in-process SDKs, ACP, structured CLI conventions, PTY fallback, real Codex/native integrations and cross-platform evidence remain pending under #185 and their canonical dependencies. No issue closure or release readiness is implied. #218 owns preventive sandbox enforcement before untrusted worker activation.

There is no persistent schema or data migration. Rollback removes this additive library from consumers; never automatically replay unresolved requests when changing versions. Accessibility is not applicable to this headless library; the production client retains its accessibility obligations. Performance evidence here concerns bounded buffers and deadlines, not the whole-process resource budgets required by #38.
