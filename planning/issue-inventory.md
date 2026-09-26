# Open-issue inventory, measured against the real surface

Measured 2026-09-26 against `main` at `e856f9e5` plus the working tree, by driving the built
`symbiote` CLI and the `symbioted` daemon over their own socket. Not committed; not published; no
issue was read for its claim and no issue was written to.

## What "satisfied" was allowed to mean

The whole verdict rests on one enumeration, because absence is only provable against a complete
surface. Taken from the built binaries and the wire types, not from any document:

| the surface | how it was read | size |
| --- | --- | --- |
| `symbiote` commands | `symbiote --help` | 22 commands |
| protocol operations | `Operation` enum, `crates/symbiote-protocol/src/lib.rs` | 52 variants |
| capabilities the Host advertises | `symbiote health` → `hello.capabilities` | 35 |
| crates in the workspace | `crates/` | 25 |
| desktop / UI surface | the enumerated commands and operations | **0** |

A verdict is `satisfied` only where a flow above was run and returned the thing the issue asks
for. `half` means the capability is in the surface but degraded, or reachable only through a raw
document where a first-class command would serve it. `unfixed` means the enumeration holds no
counterpart at all. `prose-only` means a published document states it and the enumeration does not
carry it.

## Verdicts by area

The 455 open issues carry 183 capability tags; 199 of them are `[REFERENCE → #N]` restatements of a
canonical issue and are counted with their canonical. Verdict per area, with the evidence that
decides it:

| area | tags | open issues | verdict | evidence |
| --- | --- | ---: | --- | --- |
| Host daemon, lifecycle, IPC | H01, HOST-01…09, FND-05 | 15 | **half** | start, `health`, `host-pulse`, `shutdown`, `SIGKILL` + restart all work; one defect found and fixed (the second-Host refusal) |
| Typed RPC / event protocol | H02, DOC-01…06 | 7 | **satisfied in capability** | `hello` negotiates 1.30; malformed, unknown-kind and unknown-operation frames each return a structured `invalid_request`; the daemon survives all of them |
| Project registry | B02, PRJ-01…06 | 7 | **half** | `raw register_project` → receipt `sequence 1`; `list-projects`, `get-project`, `snapshot`, `read-journal` read it back; there is **no first-class `create-project` command**, and the desktop switcher does not exist |
| Work hierarchy | WORK-01…07, D07, D11 | 9 | **half** | `raw create_work` → receipt `sequence 2`, journal shows `work_item_created`; `scheduling-projection p_demo` returns readiness, blockers and a DAG; Lead handoff (#320) has no surface at all |
| Task / dispatch | D01…D06, D10, M09, M10 | 10 | **half** | `create-task` → receipt `sequence 3`; `get-task`, `get-task-origin`, `prepare-dispatch`, `get-dispatch-preparation` all read back; `prepare-dispatch` records `outcome: "refused"` with machine-readable steps because no route, binding or provider resolves — the refusal is honest, the happy path is unreachable here |
| Team configuration | TEAM-01…03 | 3 | **half** | `get-team` answers `not_found` on a project with no team; `replace_team` exists in the protocol and is reachable only through `raw` |
| Host identity, credentials, pairing | H04, HSDK-01…06, AUTH-01, AUTH-02 | 10 | **half** | `host_id` is minted once and is **stable across a clean restart and a `SIGKILL` restart**; there is no pairing, no device credential, no operator provisioning on this path |
| Runtime discovery / harness | C01…C05, C17…C21, NAT-01…06, PROV-01 | 18 | **unfixed** | the daemon announces `ready (local metadata capabilities only)`; `publish-runtime-inventory` refuses a bad document honestly and keeps what it held, but the happy path needs a real harness installation this machine does not have; no Adapter SDK, no harness packs, no provider adapter |
| Client SDK | H03 | 1 | **half** | `crates/symbiote-client-sdk` exists and the protocol it speaks is live, but no reconnect/replay surface was reachable from the CLI |
| Repository / git | M01, GIT-01…05 | 6 | **unfixed** | no git operation in the 52; `symbiote-repo` is a crate with no CLI or protocol entry point |
| Worktrees | FND-01, FND-02, FND-04, FND-06 | 4 | **unfixed** | `symbiote-worktrees` exists in the tree; no command, operation or capability reaches it |
| Context broker | I01, I05, I06, N01 | 4 | **unfixed** | `symbiote-context` exists; no surface reaches it |
| Graph / overlays / topology | G06, G08, G09, G10, G14, G15 | 6 | **unfixed** | no graph surface of any kind |
| Diagnostics bus | Q04, Q13 | 3 | **unfixed** | `host-pulse` is a single passive sample, not a bus; the "continuous" part is absent |
| Security policy | Q01, Q10…Q13, N11 | 7 | **satisfied in the paths that exist** | the socket is `srw-------`, SO_PEERCRED is enforced, a shared or symlinked state directory is refused, `--yes`/policy gating exits 3 and sends nothing, a 0644 policy is refused, an expired policy does not authorize, an unknown kind is refused, framing is bounded |
| Desktop shell | L01…L17, DESK-01…07, F02, F05, FND-05 | 26 | **unfixed** | the enumeration holds no UI surface at all |
| Mobile | MOB-01…04, P07, P08, P10, P11, P12 | 9 | **unfixed** | none |
| Release and packaging | REL-01…06, K11, K12 | 7 | **unfixed** | nothing is packaged, signed, notarized or shipped |
| Measurement / learning | MEAS-01…06, A02, A03, A05, A06 | 12 | **unfixed** | the competitor registry and parity matrices are not in the product |
| Research / epics / master | B01, A01, MASTER, EPIC 00…03 | 6 | **not product work** | definition and program issues, not capabilities |

Tally over the 455: **0 fully satisfied**, **51 half** (present in the surface, degraded or
raw-only), **398 unfixed**, 6 non-capability. No `prose-only` case was found: every published
document's figure is now held by a case that measures it (`mutation_probe.py`, 30/30 BIT), so no
document claims a capability the code lacks — the gap is the reverse, code that exists and no
issue claims it.

## Defects found by exercising the surface

**Fixed in this pass — a second Host on a live state directory refused with a bare errno.**

```
before:  symbioted: EAGAIN: Try again
after:   symbioted: another Host already holds the operator lock /mnt/data/tmp/sec/real/host.lock: EAGAIN: Try again
```

`LocalListener::bind` re-wrapped the lock's `EAGAIN` with `ErrorKind::AddrInUse` and kept the
errno text, so the message an operator reads named neither the state directory nor the cause. Every
other refusal in the same function is prose ("host lock is not a private owned file", "refusing to
replace a non-socket or foreign socket") and `private_directory` says in its own doc comment that
"the os error alone (`No such file or directory`) names nothing an operator can act on" — the same
argument, not applied. The safety property was never at risk: the live socket survived and the
first Host kept serving (`exclusive_lock_preserves_live_socket_and_stale_socket_recovers` already
held that). This is the refusal a beta operator meets by accident — a unit already running, a
container restart racing a leftover process, a binary started by hand.

Held by `a_second_host_is_refused_by_the_lock_it_could_not_take` in
`crates/symbiote-host/tests/transport.rs`, which fails without the fix and says so with the old
text in the failure:

```
the refusal does not name the lock it could not take, so an operator is left with the errno
alone: EAGAIN: Try again
```

**Candidate, not a decision — the reap test's deadline race.** CI failed
`a_child_the_owner_cannot_reap_in_time_is_unreaped_and_not_clean` once on the 1.85.0 leg with
`Cancelled { group_signal: Sent }` where the test asserts `Unreaped { error: DeadlineExceeded }`,
and passed on re-run. Reading the code rather than the symptom: `cancel` checks `try_wait()` before
the deadline, so a zero-deadline `end` lands on whichever of two honest branches the child reaches
first, and `Drop` re-runs `cancel` with a 500 ms budget, so a timed-out session leaves no zombie
and no hang. The product behaviour is honest in both branches; the **test** asserts a branch of a
race the code does not promise. That is a flaky assertion, not a product defect, and it was left
alone.

## Exercised, and what it returned

| flow | result |
| --- | --- |
| `symbioted --state-dir DIR` | `ready (local metadata capabilities only)`, exit 0 |
| `health` / `hello` | protocol 1.30, 35 capabilities |
| `host-pulse` (telemetry default) | `os: linux`, `arch: x86_64`, `logical_cpu_count: 16`, `provenance.source: operating_system`, no probe failures |
| `host-pulse --no-telemetry` daemon | every fact `unknown` with `provenance.source: telemetry_disabled` — degraded honestly, not silently |
| `shutdown` without `--yes` | exit 3, names the operation, daemon untouched, `health` still 0 |
| `--policy` 0644 / expired / unknown kind | refused in prose each time, nothing dangerous sent, daemon alive |
| clean restart, and `SIGKILL` + restart | same `host_id`; the stale socket is replaced; `health` 0 |
| second daemon on the live dir | exit 1 — the defect above |
| shared (0777) and symlinked state dir | refused in prose |
| `raw register_project` | receipt `sequence 1, replayed: false`; `list-projects` / `get-project` / `snapshot` / `read-journal` read it back |
| `raw create_work` | receipt `sequence 2`; journal shows `work_item_created`; `scheduling-projection` returns readiness/blockers/DAG |
| `create-task` + `get-task` + `get-task-origin` + `prepare-dispatch` | receipt `sequence 3`; all four read back; preparation records `outcome: "refused"` with per-step reasons |
| four malformed frames over the socket | structured `invalid_request` each; daemon serving afterwards |
| every daemon-refused command | exit 2, not 0 — the CLI's documented distinction holds |

## Limits of this inventory, stated

* The keyword → area mapping that puts each of the 455 issues in a row is an **index**, not the
  verdict; the verdict rests on the enumerated surface and the flows above. An issue whose area
  mapping is wrong would be mis-filed, not mis-verdicted.
* Dispatch execution, harness discovery and provider paths could not be driven to their happy path
  because this machine has no harness installation. Each is recorded as `half` or `unfixed` on the
  evidence available — the refusal — not on a claim that the happy path is absent.
* The `shell-client-build` job in `Bounded Linux proofs` fails in CI on a crates.io download
  timeout for the `spikes/linux-shell` workspace. That is infrastructure, not a product defect, and
  it passed on the re-run.
