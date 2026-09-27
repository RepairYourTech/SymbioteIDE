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

**Found in the third pass — the lifecycle has three unreachable states.**

The end-to-end path now runs to `completion_requested` from an empty directory
with no `raw` file, and stops there, because nothing on the wire can move it
further. Over the whole workspace the Host constructs exactly one
`TaskAction` — `RequestCompletion`, at `service.rs:920` and twice in
`runner.rs`. `BeginVerification` and `Complete` exist in `symbiote-domain` and
are exercised by domain and store tests, and `TaskAction::Complete` additionally
requires the change stream to be `StreamState::Validated` with
`last_validated_target`/`last_validated_head` equal to the stream's target and
head — a `StreamAction` in the same file, reachable from no operation either.

So `Verifying` and `Completed` are unreachable through any client, and so is a
validated stream. A task dispatched end to end therefore stays
`completion_requested` forever: the worker's report is on record and no owner,
reviewer or Host has any way to accept it. The daemon reports this honestly —
`get_task` says `completion_requested`, and nothing else — which is why it read
as a finished run in the transcript above rather than as a gap.

This is the same shape as the two defects above, one level further out: a
record that every step that validates it accepts, and an impossibility
discovered only past the last step a client can take. It is the reason #38
("a headless Linux Host can be controlled by another desktop client") cannot be
fully satisfied by client work alone — three operations have to exist before a
controller has anything to control. Nothing in this branch touches the protocol,
so it is recorded rather than begun.

**Closed in the fourth pass — then closed again twice, one step at a time.**

The three operations exist: `validate_change_stream`, `begin_task_verification` and
`complete_task`. `Store::apply_stream` is the write path every other transition had and a
stream did not, so a stream moves at all. That closed `Verifying` and `Completed`.

The next pass found the same shape one step out: a completed task's work sat on a branch in a
worktree the Root had never seen, and no client had the operation that would have merged it.
`integrate_change_stream` does, `--ff-only`, refusing before the merge unless the root is at the
stream's recorded target and refusing after unless it is at the recorded head. That refusal is
what exposed a defect in `create-task`, which recorded the null Git object id as the stream's
target and so made a task created from the command line dispatchable, runnable, completable and
then permanently un-integratable. The target is the base now.

The step after that was the last one, and it was a leak rather than a gap: every completed run
left a full git worktree on the operator's disk forever, because nothing ever reclaimed one and
`symbiote_worktrees::release` — a function with two documented policies and a content
discipline — had no production caller anywhere in the workspace. `release_change_stream` is that
caller. It proves the stream's head is still reachable from the root (`merge-base
--is-ancestor`, so a root that moved on is fine and a root that lost the commit is not), then
runs `git worktree remove` **without** `--force` so git's own refusal of a dirty checkout is the
last line of that guard, then releases the reservation under `DeleteIfEmpty`. The branch is never
deleted: a change stream is integrated, not erased. The stream ends in a new terminal state,
`Released`, distinct from `Integrated` precisely because a caller must not read "integrated" as
"still on disk".

`symbiote-worktrees::release` could not have been that caller as it stood: `DeleteIfEmpty` read
the reserved directory and reported `NotFound` when git had already removed it, which is the
ordinary end of a materialized reservation. No materialized reservation could ever be released,
so its marker outlived its directory and `reserve` refused the identity forever. That is fixed in
the crate, with the honest reading: a location that is already gone holds no entries.

The end-to-end harness in `/mnt/data/tmp/ship` now runs the whole lifecycle from an empty
directory with no `raw` file: `init`, `add-credential`, ten registrations at sequences 1–10,
dispatch, run, `validate` (15), `begin` (16), `complete` (17), `integrate` (18, revision 2) and
`release` (19, revision 3) — after which the reservation base is empty and the branch is still
there.

**Found in the fifth pass — the happy path was complete and a refused run was a dead end.**
With the lifecycle walkable, the harness was rewritten to run three tasks in a row, because a path
that works exactly once is not a product. That passed: three distinct worktrees, three branches,
an empty reservation base after each release, one `git worktree` registration left in the root.
Then the same harness asked the question a user asks constantly — what does a *refused* run say?

The refusal is one a person causes by accident: they commit to their own repository between
creating the task and dispatching it, so the stream's recorded base is no longer where the root
is, and `provision_worktree` refuses with `BaseMoved` before any worker exists. The transcript
then read:

```
run-started-dispatch {"code":"failed_precondition","message":"worktree provisioning refused: BaseMoved"}
state afterwards = running
```

`running`, with nothing running. Over the whole workspace the Host constructed four `TaskAction`s
and `Fail` was not one of them, so a task that entered `Running` could only leave it by completing
— and a run that refused pinned it there for the life of the store. Worse, four tests *asserted*
`running` after a refusal, one of them with a comment calling the state "intact for Host retry
policy". That assertion was the defect: the retry policy those tests meant to protect is the
dispatch binding, which does survive; the task's own state was never the part that had to hold.

The second blocker was a consequence of the release commit above. The reservation the refused
provisioning left could not be reclaimed by anything: `release_change_stream` requires an
integrated stream, and this stream never becomes one. It answered
`worktree removal refused: GitRefused` — because the directory existed and git had never heard of
it. That refusal means "nothing to remove"; reading it as "work in the way" is the mirror image of
reading a dirty worktree as an empty one.

Three operations close it, and they are one story. A refused run now files its own `Fail` before
returning the refusal — every error past the two request-identity preconditions, under a command
id derived from the run's own — and releases the lease, which was a second claim that execution
was in progress and which fenced the task's own re-dispatch with `resource conflict`.
`abandon_change_stream` cancels an unlanded stream and reclaims its location, and is the only
operation in the product that destroys work: it refuses an integrated stream outright, and
`--force` is a parameter (`RemovalForce`) rather than a property of the repository crate, because
only the abandon may discard uncommitted files and only the release may not. `recover_task` returns
`Failed` or `Interrupted` to `Ready`; the lifecycle reaches `Ready` only through it, which is why
the two were missing together.

`symbiote_repo::registered_worktrees` is the fact the reclaim path turns on, and it exists because
conflating "no worktree" with "dirty worktree" in either direction is wrong. `reclaim_worktree` is
one function both paths call, so the integrated and abandoned cases cannot drift apart in the only
respect that matters: what is deleted. Both ask their state question *before* the irreversible
half — the first version of the abandon answered its own second call with
`worktree reservation release refused: NotFound`, having deleted nothing and then reported a
filesystem error instead of the lifecycle answer.

The second harness transcript, `/mnt/data/tmp/ship/failing_run.sh`, walks it with first-class
commands only and ends with a new task running to completion.

**And the report that could have said so, did not.** `prepare_dispatch` on that same recovered
task returned `outcome: "ready"` — and then the start refused with `stream_inactive`. The report's
`Worktree` step was `declared: true`, a constant that could not be false, and the refusal the
boundary used was `DispatchRefusal::StreamInactive`, a vocabulary member *no report could reach*.
The same shape as the three defects above, one level down: a step reporting a fact it never
checked. The step now carries `startable` and the `state` it read, the outcome accounts for it, and
`validate` refuses a report whose step contradicts its own recorded state — so nothing can hand a
client a report calling a cancelled stream startable.

**The same path from a second cause, and a probe I had to throw away.** Setting the fixture's
report to empty looked like it would produce a run that "completed" with nothing to show, which
would have been a sixth defect. It did not — the domain refuses a completion whose report is empty
— but the first probe answered `completed: true` anyway, because the daemon had cached the
operator configuration at startup and I had edited the file without restarting it. The lesson is
the harness's own: a probe that contradicts the product deserves a check of the probe first.
Restarted, the run halts with `worker loop halted without finishing` and the task reads `failed`,
which is the new path working from inside the loop rather than at provisioning. That case is now
pinned by `a_run_that_halts_inside_the_loop_is_recorded_and_its_worktree_is_abandonable`, which is
the half the provisioning test cannot reach: git *knows* about the location, so the abandon
removes a real registered worktree — with uncommitted content in it, which is what the release path
refuses and only this path may discard — and leaves neither the directory, nor the marker, nor the
administrative registration in the root's `git worktree list`, while the branch survives. A mutant
that skips the removal when the location is registered dies on it with
`worktree reservation release refused: NotEmpty`, which is the second guard doing its own job: a
populated directory is never deleted just because the marker says it may be.

**The note the fifth pass said was missing, and the sixth pass's last change.** The abandoned
stream is terminal, so the retry is a *new* task — a recovered task cannot be re-dispatched
under a cancelled stream. That is the correct answer rather than a gap, but it means a failed
unit of work costs a new task record, and while the preparation now says `stream_inactive` up
front, nothing told an operator that up front *at create* time. The CLI now says it, on stderr,
from the six commands where a step ends a task, a run or a stream: `create-task` names the cost
before it is incurred, and `abandon-change-stream`, `release-change-stream`, `complete-task`,
`interrupt-task` and `recover-task` each name theirs when they charge it. It is stderr rather
than a field because `--json` prints one envelope per invocation and that envelope is a
published contract; it is printed only on success, because nothing was charged when a command
was refused, and not on reads, because a note on every command is a note nobody reads. A
mutant that moves the note into the envelope fails
`a_created_task_says_out_loud_what_abandoning_its_stream_will_cost`, and one that prints it on a
refusal fails on the same line — the refusal half of that test is an undispatched task, which
cannot abandon its stream at all.

**The fourth transcript: two Projects on ONE daemon, first-class commands only.**
`/mnt/data/tmp/ship/two_projects.sh` registers `alpha` and `beta`, each with its own Root, team,
binding, credential reference, worktree and branch, on one daemon, and runs both to completion.
What it shows, in the order the boundary matters: each dispatch compiled its own project's
`effective_access` — project, root and credential reference all from its own Project, which is
where the isolation actually lives rather than in a convention; `get-task beta task-alpha` is
`permission_denied` and a run naming another task's dispatch is `failed_precondition`; a binding
registered against a Project that does not exist is `not_found`. Two facts the daemon will not
absorb, both `conflict` with the terse message the protocol's vocabulary owns: **role ids and
work ids are global**, so the second Project's Objective and its Roles carry their own names. And
landing one Project's work left the other's alone — both released, the reservation base empty,
both branches surviving their checkouts, and neither Root keeping a stale `git worktree`
registration. The transcripts are first-class commands end to end, with no `raw` file anywhere.

**Found in the sixth pass — the crash, which is the one failure nothing can file for itself.**
The two transcripts above are runs that lived long enough to see their own refusal. A `kill -9`
mid-run is the other thing: the `host_id` survives and the store is durable, so the daemon comes
back and a re-dispatched `run_started_dispatch` replays to `completion_requested` — but a run the
operator never retried sat at `running` for the life of the store, holding a lease and a checkout,
with no process behind either. `recover_task` refused it: a recovery accepts `Failed` and
`Interrupted`, which are states a *run* files about itself. And the abandon answered
`worktree reservation release refused: NotFound` — the reclaim had already removed the location in
an earlier attempt, and the retry reported a filesystem error for work the caller had done.

Three things close that, and the third was the real one.

`Operation::InterruptTask` is the operation for the case nothing else can reach, with
`interrupt-task <task_id> <reason>` in front of it, classified dangerous like the rest of the
execution-ending operations. The reason is a required argument rather than a default because the
Host cannot tell a crash from an operator who walked away from a terminal, and this record is the
only place that survives to whoever reads it next. It files `TaskAction::Interrupt` as the Host and
releases the lease, which is what makes the recovery that follows possible at all: a task marked
`interrupted` and fenced by a lease held by a dead process is the same dead end one layer down.
`reclaim_worktree` is idempotent, because an absent marker *is* the reservation crate's own
definition of released — a directory left behind without one is an abandoned location it never
reclaims, and a tamper signal (a marker that exists and cannot be read) is not absorbed.

And then the re-dispatch refused `worktree provisioning refused: Reservation`, which is the finding
that mattered. A Task has exactly one Change Stream for its whole life, so a recovered task can
only ever be dispatched again against the stream it has — and provisioning refused a location the
stream had claimed itself. `recover_task` was therefore a trap: it moved a task from `failed`, with
a readable reason and a reclaimable checkout, to `ready`, and every run from there on refused. A
task that can only be run once is not a task with a recovery operation. `provision` now asks
whether the stream still holds the location (`verify_reservation`, the identity half of `verify`
with the emptiness rule left out, because emptiness is a premise for handing a directory to `git
worktree add` and not a property of holding it) and reuses it only when git registers a worktree
at exactly that path whose HEAD is the stream's derived branch — four proofs, all of which the
first claim made, and content that fails any of them is refused exactly as before. A held location
with nothing in it is not a reuse case but a claim whose materialization never happened, so it
materializes as a first run does: the operator who restores a moved base and runs again now gets a
checkout instead of `AlreadyReserved`.

The same pass found the abandon asking its questions in the wrong order. It is the one operation
in the product that destroys work, and it reclaimed the checkout *before* discovering that a
recovered task has no dispatch binding to journal the cancellation under — a refusal the caller
could not undo, on a stream nothing could then cancel. Both preconditions now precede the
irreversible half, and `an_abandon_that_cannot_journal_its_decision_removes_nothing` pins it with
an uncommitted file in the checkout: a mutant that puts the reclaim back first dies on
`a refused abandon removes nothing, and it means nothing`.

The third transcript, `/mnt/data/tmp/ship/crash_run.sh`, walks the whole thing with first-class
commands only: a halted run, a re-dispatch, `kill -9`, a restart, the task still `running` with
the dead run's output on disk, `recover-task` refused, `interrupt-task` with a reason, the empty
reason refused by name, `recover-task`, and the same task running to completion under the same
stream in the same checkout.


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
