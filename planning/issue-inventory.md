# Open-issue inventory, measured against the real surface

Measured 2026-09-26 in two passes by driving the built `symbiote` CLI and the `symbioted` daemon
over their own socket. The first pass measured `main` at `e856f9e5`; the second re-measured the
write paths and the argument surface on this branch, which carries #764's three first-class write
commands. Where the two passes measured different things, the table says which pass measured what.
No issue was read for its claim and no issue was written to. Committed here; not published.

## What "satisfied" was allowed to mean

The whole verdict rests on one enumeration, because absence is only provable against a complete
surface. Taken from the built binaries and the wire types, not from any document:

| the surface | how it was read | size |
| --- | --- | --- |
| `symbiote` commands | the rows `symbiote help` lists | 25 commands |
| protocol operations | `Operation` enum, `crates/symbiote-protocol/src/lib.rs` | 52 variants |
| capabilities the Host advertises | `symbiote health` → `hello.capabilities` | 35 |
| crates in the workspace | `crates/` | 25 |
| desktop / UI surface | the enumerated commands and operations | **0** |

Three commands were added to that surface since the first pass: `register-project`, `create-work`
and `create-task`, which took the count from 22 to 25. The protocol operation count did not move,
because those three commands send operations that already existed and already returned receipts —
this is a client surface, not a protocol change. The other three rows were re-read in the second
pass and are unchanged.

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
| Project registry | B02, PRJ-01…06 | 7 | **half** | `register-project` → receipt `sequence 1`; `list-projects`, `get-project`, `snapshot`, `read-journal` read it back. The **registry write is now first-class**; the dropdown switcher and the per-Project operating context both halves of B02 and PRJ-01 also ask for are still absent, which is what holds the area at `half` |
| Work hierarchy | WORK-01…07, D07, D11 | 9 | **half** | `create-work` → receipt `sequence 2`, journal shows `work_item_created`; `scheduling-projection p` returns readiness, blockers and a DAG. Filing an Objective is first-class; Lead handoff (#320) and D11 have no surface at all, and a request, plan, milestone or checkpoint work item is still `raw`'s |
| Task / dispatch | D01…D06, D10, M09, M10 | 10 | **half** | `create-task` → receipt `sequence 3` from its arguments; `get-task`, `get-task-origin`, `prepare-dispatch`, `get-dispatch-preparation` all read back; `prepare-dispatch` records `outcome: "refused"` with machine-readable steps because no route, binding or provider resolves — the refusal is honest, the happy path is unreachable here |
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
raw-only), **398 unfixed**, 6 non-capability. No `prose-only` case was found. The numbers are
unchanged by the three new commands, and the reason is worth stating plainly rather than rounding
in their favour: **no issue's verdict moves.** What moves is the *evidence* in three rows, and the
half of an issue that a command delivers.

### Which issues the new commands move, and which they do not

Named against the capability tags, so this is checkable rather than a claim about the work:

* **B02, PRJ-01** (Project registry and the atomic full-context workspace switcher). The registry
  write is first-class now: a Project, its lead Role and its Roots are registered from a command
  line. The switcher half — the dropdown, and switching a live context across repositories — is
  untouched, so the issue stays `half`.
* **PRJ-02** (logical Projects with multiple repository, documentation, infrastructure and
  non-repository roots). `register-project` takes more than one root, so a multi-root Project is
  registerable from the command line. The `.symbiote` manifest family PRJ-02 and PRJ-03 also name is
  still absent, so neither issue moves off `half`.
* **WORK-01, D07** (Client Request, Objective, Capability, Goal, Task and engagement semantics).
  Filing a classified Objective is first-class, and `create-task` names one. The rest of the
  semantics — a request with an utterance, a plan, a milestone, a checkpoint, engagement state — is
  still `raw`'s or absent, so both issues stay `half`.
* **D01** (per-Project Team and Role configuration). A Project is registered with its lead Role,
  but `get-team` still answers `not_found` until a team is replaced, and `replace_team` is still
  `raw`-only. D01 stays `half`.
* **D10** (Task graph status, progress, blocking, health) is read through
  `scheduling-projection` and is unchanged.

Nothing else in the 455 is touched by a client-side command: the areas that are `unfixed` are
`unfixed` because no command, operation or capability reaches them at all, and adding a client
surface for an operation that does not exist cannot change that.

## Defects found by exercising the surface

**Fixed in the first pass — a second Host on a live state directory refused with a bare errno.**

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

**Fixed in the second pass — a request that was never sent was reported as an unreachable daemon.**

```
before:  symbiote: cannot reach the daemon at /mnt/data/tmp/cli-adv/state/host.sock: request must be a
         single bounded JSON frame (is symbioted running with --state-dir /mnt/data/tmp/cli-adv/state?)
after:   symbiote: the request for create-work is 66022 bytes, past the 65536-byte request bound;
         nothing was sent (the command line's own arguments are the whole request)
```

A 64 KiB title is under the kernel's per-argument ceiling, so an operator can reach this with one
keystroke too many, and the message named a cause that was not the cause (the daemon was serving)
and offered a remedy for a daemon that needed none (start it). The bound was already published and
already enforced; what was wrong was only the sentence, and the envelope code, which claimed
`unreachable` for a request that never left the process. The bound is now checked where the
message is written, before the socket is opened, and refused as `request_refused`.

**Fixed in the second pass — an argument position that was used before it was asked for.**
`register-project` read its roots by slicing the argument list, so a command line that stopped
before the first root panicked instead of refusing. Every argument position of every command is
now driven one at a time by `every_argument_position_is_asked_for_before_it_is_used`.

**Fixed in the second pass — an empty argument reached the daemon.**
`register-project "" Name lead contract@1 root` produced `invalid_request` — a typed refusal that
names neither the field nor the value, for a field the CLI had in hand. An argument that is there
and empty is now refused by name, like a missing one, because no identity, name, title or branch
can be empty. What an argument may *contain* stays the domain's and still comes back as its own
typed refusal.

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

Two tables, because the two passes measured different things and the difference is not worth
hiding. This branch's only product change is in the `symbiote` client — no daemon, protocol,
store, schema or envelope changed — so the daemon's rows are carried from the first pass unchanged
rather than re-driven, and the client's write rows are the second pass's own.

**The write paths and the argument surface, measured on this branch:**

| flow | result |
| --- | --- |
| `register-project` | receipt `sequence 1`; `list-projects` / `get-project` / `snapshot` / `read-journal` read it back, roots in the order they were named |
| `create-work` | receipt `sequence 2`; journal shows `work_item_created`; `scheduling-projection` returns readiness/blockers/DAG |
| `create-task` + `get-task` + `get-task-origin` + `prepare-dispatch` | receipt `sequence 3`; all four read back; preparation records `outcome: "refused"` with per-step reasons |
| identifiers at the domain's edges | 1 and 128 characters accepted; 129, `/`, a space and an empty string each refused — the empty one now by name, the rest as the domain's `invalid_request` |
| empty and whitespace-only names | empty refused by name; whitespace-only is the domain's rule and is answered as its typed `invalid_request` |
| spaces, quotes, tabs, newlines, non-ASCII in a title, description, name or branch | accepted; the domain refuses only a NUL. A branch with a control character, a branch past 512 bytes and a title past 256 are the domain's `invalid_request` |
| a surplus argument | refused by name (`create-work does not take the argument "surplus"`), never dropped |
| an argument that begins with `--` | read as a flag, as documented; `--` ends flag parsing and the help now says so |
| 33 roots in one `register-project` | the protocol's own limit, refused as `invalid_request` |
| a 1 MiB argument | refused by the kernel before the CLI runs (`Argument list too long`, exit 126) |
| a 64 KiB argument | refused as the request it is, with its own code — the second defect above |
| `--command-id` replayed with the same arguments | the same receipt with `replayed: true` |
| `--command-id` replayed with different arguments | `idempotency_conflict`, "command identity was reused for different intent" |
| eight clients registering eight Projects at once | eight receipts, sequences 17…24, no gap, no duplicate |
| eight clients registering the same Project at once | exactly one receipt, seven `conflict`, one journal event, daemon serving |
| a client `SIGKILL`ed inside its own lifetime, 200 rounds (178 landed) | the daemon answered `health` after every one; 177 retries replayed the committed receipt and 23 filed it; zero conflicts, zero Projects registered twice, and every Project's journal held exactly one registration |
| every daemon-refused command | exit 2, not 0 — the CLI's documented distinction holds |
| `symbiote schema --check docs/contracts/schemas` | exit 1 naming the file and line when the published envelope description moved, 0 after `schema --write` regenerated it |

**Everything else, measured in the first pass against `main` at `e856f9e5` and unchanged here:**

| flow | result |
| --- | --- |
| `symbioted --state-dir DIR` | `ready (local metadata capabilities only)`, exit 0 |
| `health` / `hello` | protocol 1.30, 35 capabilities |
| `host-pulse` (telemetry default) | `os: linux`, `arch: x86_64`, `logical_cpu_count: 16`, `provenance.source: operating_system`, no probe failures |
| `host-pulse --no-telemetry` daemon | every fact `unknown` with `provenance.source: telemetry_disabled` — degraded honestly, not silently |
| `shutdown` without `--yes` | exit 3, names the operation, daemon untouched, `health` still 0 |
| `--policy` 0644 / expired / unknown kind | refused in prose each time, nothing dangerous sent, daemon alive |
| clean restart, and `SIGKILL` + restart | same `host_id`; the stale socket is replaced; `health` 0 |
| second daemon on the live dir | exit 1 — the first defect above |
| shared (0777) and symlinked state dir | refused in prose |
| `raw register_project` / `raw create_work` / `raw <task.json>` | receipts at sequences 1, 2 and 3: `raw` still carries every field the typed commands derive, which is what it is for |
| four malformed frames over the socket | structured `invalid_request` each; daemon serving afterwards |

## Limits of this inventory, stated

* The keyword → area mapping that puts each of the 455 issues in a row is an **index**, not the
  verdict; the verdict rests on the enumerated surface and the flows above. An issue whose area
  mapping is wrong would be mis-filed, not mis-verdicted.
* Dispatch execution, harness discovery and provider paths could not be driven to their happy path
  because this machine has no harness installation. Each is recorded as `half` or `unfixed` on the
  evidence available — the refusal — not on a claim that the happy path is absent.
* The daemon's own refusals are terse where they are the daemon's to make: a duplicate Project, a
  duplicate Role id (role ids are global, so a second Project cannot reuse one) and a duplicate
  work id are all `conflict` with the message "resource conflict", and a missing Project, Root,
  Role or work item is `not_found` with "resource not found". Nothing names which resource. That
  is a finding, not a fix: the error vocabulary is the protocol's, and changing it is a protocol
  change rather than a client one.
* `shell-client-build` in `Bounded Linux proofs` has failed in CI on a crates.io download timeout
  for the `spikes/linux-shell` workspace. That is infrastructure, not a product defect, and it
  passed on the re-run.
