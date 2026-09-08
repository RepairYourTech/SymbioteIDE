# Linux Host lifetime probe — bounded #38 evidence

Owner: #38. This standalone experiment is in `spikes/host-lifecycle`; it does not select production persistence or protocol, replace #43/#181/#180, or implement the canonical Host. The binary launches only its own fixed root → child → grandchild workload. It accepts no command, path, credential, provider or remote network target from a client.

## Mechanism and demonstrated boundary

The daemon is a separate Rust process. A mode-0700 directory owned by its current user contains a mode-0600 Unix socket, exclusive advisory lock and atomic JSON journal. Clients send one newline-delimited JSON request per connection. Requests are limited to 4096 bytes with read/write timeouts; malformed and unknown fields are rejected. A second daemon cannot replace the first daemon's socket while it owns the lock.

The fixed workload is placed in its own process group with `Command::process_group(0)`. The daemon retains its root `Child` and an independent stdin lifetime pipe. It writes the root PID, Linux start ticks and unique run token to the journal **before** sending the authorization handshake that lets the root create descendants. Closing a client connection does not close this pipe. The root handles daemon EOF by terminating its own process group.

Cancellation checks the root's `/proc` start ticks, group membership, fixed-workload token and unreaped owned `Child` before `killpg`. The owned child pins PID reuse across the signal operation. It waits for the group's live token-bearing root/child/grandchild processes to disappear and reaps the directly owned root. This is process lifetime evidence, not sandboxing or proof that arbitrary external agents cannot escape a process group.

Journal updates write a new file in the same private directory, sync the file, rename it atomically, and sync the directory. Restart validates the version and contiguous event sequence, preserves existing events, and observes the prior group for a bounded three seconds. It **never signals PIDs from a restored journal**. If no live owned workload members remain, it records `recovery_confirmed_exit`; otherwise it records `recovery_required`, rejects new starts/cancellation and keeps the unresolved identity visible. A corrupt journal is preserved and refuses startup.

Daemon SIGKILL is tested with a real running three-process workload: its pipe guardian cleans up, and restart retains event cursors. The deliberate `ignore_guardian_eof` fault also runs real processes: they survive daemon exit, restart reports `recovery_required`, and automatic cleanup does not falsely pass. Only the owning test cleans up this injected workload. **Unconditional orphan cleanup is not proven:** this strategy depends on a functioning guardian and fixed non-escaping workload. Production supervision needs a separate proven containment/recovery mechanism before that hard requirement can pass.

Fault-test cleanup writes a stop marker inside its owned private directory. The still-live workload root observes that control and signals its own group; the test never signals a scanned or restored PID. This removes the PID-reuse race that a test-side token check followed by `killpg` would otherwise have after daemon exit.

## Reproduce and optional client integration

From the repository root:

```sh
cargo test --locked --manifest-path spikes/host-lifecycle/Cargo.toml
cargo clippy --locked --manifest-path spikes/host-lifecycle/Cargo.toml --all-targets -- -D warnings
```

To inspect manually, create an owned private temporary directory and run the built binary with `serve PRIVATE_DIRECTORY`. Connect to `PRIVATE_DIRECTORY/host.sock`. Normal requests:

```json
{"command":"start"}
{"command":"status"}
{"command":"events","after":0}
{"command":"cancel"}
{"command":"shutdown"}
```

Responses contain `ok`, `status` (`idle`, `running`, `recovery_required`), monotonically increasing `cursor`, optional owned `group`, replayed `events` and optional `error`. `events.after` is exclusive; a cursor ahead of the journal is rejected. `shutdown` explicitly cancels active work; client EOF alone does not. The fixed fault injection `{"command":"start","fault":"ignore_guardian_eof"}` is for the automated negative test only and intentionally requires test-owned cleanup after daemon death. It is not a production API.

This protocol is available for optional shell experimentation. No Tauri integration or UI close/crash scenario is claimed by this standalone CLI-client proof.

## Captured evidence

Executed on Linux `7.2.2-arch1-1`, x86_64 GNU/Linux, Rust `1.97.1 (8bab26f4f 2026-07-14)`. The standalone package declares Rust 1.85 and exact direct dependency versions; `Cargo.lock` pins the resolved graph. Rust 1.85 execution is not independently verified here.

Raw final integration-test output:

```text
running 7 tests
test nonprivate_directory_is_rejected ... ok
test malformed_requests_and_future_cursors_do_not_launch_work ... ok
test invalid_journal_is_preserved_and_second_daemon_cannot_take_socket ... ok
test stale_journal_identity_never_signals_an_unrelated_live_process ... ok
test daemon_sigkill_closes_guardian_lease_and_restart_preserves_event_cursor ... ok
test two_clients_disconnect_reconnect_and_cancel_entire_fixed_process_tree ... ok
test unresponsive_guardian_blocks_recovery_without_blind_cleanup ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.08s
```

Strict Clippy completed successfully with `--all-targets -- -D warnings`. The malformed-request test initially exposed Serde unit-variant unknown-field acceptance; fixed struct variants now reject injected command fields. No skipped test is counted as passed.

| #38 concern | Bounded result |
| --- | --- |
| Client disconnect leaves authorized headless work alive | Passed for two real local clients and fixed three-process workload |
| Reconnect and cursor replay | Passed across independent client connections and daemon restart |
| Explicit process-tree cancellation | Passed for fixed non-escaping root/child/grandchild group |
| Daemon crash cleanup and recovery | Passed with functional EOF guardian; injected guardian failure remains visible and blocks new work |
| PID reuse / unrelated process safety | Restart never signals journal PIDs; stale identity fixture deliberately references the live test runner |
| Atomic journal replacement | Sync/rename mechanism exercised during tests; power-loss/filesystem-failure guarantees untested |
| Arbitrary external runtimes, PTYs, escaped descendants, concurrent process failures | Not proven; remain required downstream |
| Production durable task state and transaction semantics | Not implemented; this bounded journal is not the persistence decision |
| Bounded production event retention/backpressure | Not proven; in-memory history is suitable only for the small experiment |
| Shell exit/crash, Windows/macOS, resource benchmarks | Not exercised by this probe |

Tests create uniquely named private temporary directories and remove only their own artifacts. `/proc` scans are observational; signals target only the captured fixed workload group or the directly owned daemon. Live-process checks exclude zombies; the daemon reaps its root, while orphan descendants depend on the OS reaper. No claim of zero transient zombies or whole-system cleanup is made.
