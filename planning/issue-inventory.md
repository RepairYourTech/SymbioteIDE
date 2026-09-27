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


**Found in the seventh pass — a change stream that could not carry a change.** The
sixth pass's second test had to commit work into the stream's own checkout to have something to
integrate, and it refused with `lifecycle refused the transition (StaleEvidence)` at
`validate_change_stream`. Reading that from the domain rather than from the symptom:
`StreamAction::Validate` compares the head the caller observed against `stream.head()`, and
`stream.head()` is set once — at `create_task`, to the base the work was to start from. So the
only commits any task could ever validate against were the ones it did not make. And the action
that would move it, `StreamAction::Advance { head, target }`, had been in the domain since the
lifecycle was written, with unit tests in `symbiote-domain/tests/contracts.rs` and **no
constructor anywhere in the workspace**: the protocol had no operation for it, the Host no arm,
the CLI no command. `grep -rn "StreamAction::Advance" crates/` returns the domain's own match
arm and its tests, and nothing else.

The consequence is the whole point of the product. Every transcript and every daemon test that
walked the lifecycle to `integrate` and `release` did so with a run that **committed nothing** —
the fixture lane reports a string and writes no file — so the lifecycle was walkable end to end
and no run's work could ever reach a Root. A task that did something sat at
`completion_requested` with `validate` refusing it forever, and since `complete` needs a
validated stream and `release` needs an integrated one, there was no way forward. The demo proved
the shape of the thing and hid the only fact that mattered.

`Operation::AdvanceChangeStream` is that missing operation, and it names **no commits**: the Host
reads the stream's checkout for the work and the Root for where it lands, and refuses a checkout
whose history does not contain the stream's recorded head — a stranger's commits wearing this
stream's path are the one thing the stream's head must never be made of. The advance invalidates
earlier evidence by construction (`Advance` clears `last_validated_*` and returns the stream to
`NeedsRevalidation`), so `validate` has to run again, and the CLI says so on stderr. Held by
`a_stream_only_advances_onto_work_built_on_its_own_head`, which commits in the checkout, advances,
validates, and then orphans the branch and proves the refusal —
`the stream's checkout is not built on the stream's recorded head` — is the *only* thing standing
between an operator and a stream that claims whatever is in its directory. A mutant that drops the
`is_ancestor` proof dies on that line with an `Ok` receipt where the refusal belongs.

The same test walk found the integrate is not idempotent against its own effect, which is the
wedge the sixth pass fixed one step earlier. The merge lands on the Root; the journal write that
turns the stream `Integrated` is a separate step after it; a daemon killed in between leaves the
work landed and the stream `Validated`, and the retry refused with `root checkout is not at the
stream's recorded target` — with the release needing an integrated stream and the abandon
refusing a stream whose work landed. The Root being at the stream's head is the same fact the
operation exists to prove, so it is now accepted as the state it means. A mutant that restores
the old comparison dies on `step journal-integrate failed: ... "root checkout is not at the
stream's recorded target"`.

And the two-project transcript now commits. `two_projects.sh` was landing two streams whose runs
had written nothing; it now commits in each checkout and advances each stream, and the ending
reads `repo: HEAD is 697b1d1 the run's work` for both Roots — the promise the product makes, with
first-class commands and no `raw` file.

**Found in the eighth pass — a task that could be run but never closed.** The sweep that
found `StreamAction::Advance` is the one worth repeating: enumerate every variant of the two
lifecycle action enums and grep for a production constructor. `TaskAction::Cancel { reason }` had
unit tests in `symbiote-domain/tests/contracts.rs` and **no constructor anywhere in the workspace**
— `production=0`, same signature as the advance. So `TaskState::Cancelled` was unreachable, and
`docs/contracts/work-hierarchy.md` counts `Cancelled` as *closed* against `open`: the one state
that closes a task was the one state the product could not reach.

The symptom is not a crash, it is a task that will not go away. A Task is created `Ready`, and
`Ready` and `Assigned` are the states `TaskState::is_startable` reports, so it stays in the
scheduling projection for the life of the store. An operator who abandons a stream and then wants
the task itself gone has nowhere to put it: `abandon_change_stream` cancels the *stream*, and the
only way to "run it again" is a dispatch into the stream the abandon already cancelled, which
`DispatchRefusal::StreamInactive` refuses. Every retry is the same refusal.

`Operation::CancelTask { task_id, reason }` closes it, and the design question was where to put
the gate. The obvious place — the state — is wrong, because the states a cancel legitimately takes
are exactly the ones a live run occupies. The gate is the **hold**: a cancel is a claim about a
Task, not authority over a directory, and the directory is the one thing here that can lose
commits and uncommitted files. So the arm reads the stream's reservation marker and refuses with
`the task's stream still holds a reservation; abandon-change-stream reclaims it` while one is
present, naming the operation that has the authority the cancel does not. Measuring that produced
the asymmetry that makes the operation usable: **provisioning takes the reservation, `create_task`
does not**. A Task nobody ever dispatched holds nothing and closes in one call; a Task whose run
provisioned holds a checkout and needs the abandon first. That was not a guess — the first draft
of the test asserted the refusal on a freshly created task and the daemon cancelled it, because
there was no marker to refuse on.

The same test then found a defect in the draft. `workers.reservation_base()` was propagated with
`?`, so on a Host with no `reservation_base` configured — which is what `Host::new()` gives the
real-binary tests — the cancel refused with `no worktree reservation base configured`. That is
backwards: a Host with no base cannot be *holding* a reservation, so the absence is the answer and
not a refusal, and propagating it made the operation unusable on exactly the Host an operator is
most likely to be talking to. The base is worker configuration; every path that truly needs it has
to provision, and provisioning is where that refusal belongs. It is now `.ok()` into the same
predicate, so the check is total: no base, no project directory, or no matching marker all read
as "not held", and only a marker for this stream is a hold.

Held by `a_task_is_closed_only_after_its_stream_gave_up_its_checkout`, which walks both halves
against a real repository: a never-dispatched task closed in one call, a run that provisioned
writing an uncommitted file into its own checkout, the refusal twice over (once for a task that
was never dispatched *and* never provisioned is not asserted — the one that matters is the one
with a real dispatch behind it), the abandon reclaiming, the close, and then terminality from both
sides: a cancelled task is refused by `recover_task` and by `start_prepared_task`. Three mutants
die on it. Dropping the hold check entirely dies on the refusal line with a task that went
`completion_requested` → `cancelled` while `the-work.txt` was still on disk. Making the marker
match never fire dies on the same line. And skipping the lease release — which is *why* a
cancelled task's own restart is refused with `failed_precondition` rather than as a live run — dies
on that restart.

`cancel_task` is classified **dangerous**, not merely a mutation, and for the same reason
`complete_task` is: `Cancelled` is terminal and the lifecycle recovers only `Failed` and
`Interrupted`, so a policy has to be told about a step that cannot be undone. It is the ninth
dangerous kind in the table, and both pin lists in the CLI's own tests move with it.

The orphan sweep now returns six, down from seven: `TaskAction::{Queue, Assign, Block, Unblock}`
and `StreamAction::{Collision, Reconcile}`. The four task actions are the pre-dispatch states the
lifecycle already models — `DispatchRefusal::NotSchedulable` and `is_startable` both exist to
read them — and nothing moves a task into one, so a task is either run or not run. The two stream
actions are the Root moving under an active stream, which is a real operator event with no
operation at all.

**Found in the ninth pass — a gate the report did not know about, and a refusal that
named nothing.** `TaskDependencyKind::Requires` says in its own doc "This task cannot
start until the target completes", and the boundary enforced it. So the orphan sweep's
next question was not whether the gate exists but what an operator is told when it fires,
and the answer was measured rather than guessed: `{"code": "conflict", "message":
"resource conflict"}`.

Two controls established that the refusal was the edge and not the environment, because a
refusal proves nothing until you know what it is refusing *instead of*. A task with no edges
started. A second task with no edges started while the first was running. The task whose
`requires` target was not `completed` was refused. So the enforcement was real and the
answer was useless.

`StoreError::DependenciesUnresolved` was a **unit variant**. It could not say which
prerequisite, and it mapped to `ErrorCode::Conflict` — the code this product uses for two
writers racing for the same row. Three gates reach that check, from either direction: a
`requires` target that has not completed, a `blocks` edge another task has put on this one,
and a task whose prerequisite is *cancelled* rather than running. The three have three
different remedies — wait, drop the edge, or give up on the task — and the refusal named
none of them. The line above it in the same function got it right: the stream check raised
`DispatchRefused(StreamInactive)` three lines below. The dependency check was the one hole
in an otherwise careful design, and the fix is the mechanism already there.

The report was wrong in the same place, and that half mattered more. `prepare-dispatch`
returned `outcome: ready` with `{"kind": "scheduling", "schedulable": true}` for the very
task the boundary was about to refuse, because the composition read the task's own state
and not what was holding it. `CompositionStep::Worktree { startable, state }` exists
precisely to stop that class of defect, and states the rule in its own doc comment: the
boundary "refuses every state but `Active` with `stream_inactive`, so the report has to be
able to say the same thing — otherwise it reports a dispatch the start is about to refuse."
The same argument had never been applied to the dependency gate.

So there is now a `dependencies` step naming every unsatisfied gate and the state it is
in, and `DispatchRefusal::DependencyUnresolved` whose wire name is the
`dependency_unresolved` the scheduling projection **already published** for the same fact —
so one word answers in the projection, the report and the boundary. The composition reads
`unsatisfied_gates`, the same function the boundary refuses on; two reads would be two
chances to answer different questions, which is what the store's own
`scheduling_projection` doc says about the dependency verdict it does not re-derive.

The error itself now carries `Vec<TaskGate>` rather than nothing, so the *completion* path
names the gates too (`a recorded gate is not satisfied: demo/prereq is ready; nothing runs
until it is`) and uses `failed_precondition` — a condition the caller has not met, not a
conflict it lost.

**And the gate was unrecordable from the client.** `set_task_dependencies` and
`get_task_dependencies` were in the protocol with no typed command at all, so the one
relation that holds a task back from starting could only be set through a `raw` operation
file — on a branch whose whole subject is first-class commands. That is why the first probe
of this finding needed one. `set-task-dependencies <project> <task> <kind>:<task>[,...]`
and `get-task-dependencies` are the commands now, taking the whole edge set rather than a
delta, because the store's contract is replacement semantics and a caller that thought it
was adding an edge would silently be dropping the others.

That command then shipped a defect of its own, caught by the transcript rather than by a
test. It narrowed to the two enforced relations by mapping over a lookup's `Err`, which
asks "is this **none** of them?" — and the five provenance kinds *are* in the spelling
list, so `reviews` was accepted and stored. An edge recorded where nothing reads it reads
back to an operator as a gate that was accepted: the same defect as a report promising a
dispatch the boundary refuses, one layer down and in a command that did not exist an hour
earlier. Only the positive form works, and
`a_typed_dependency_command_refuses_a_relation_no_boundary_enforced` holds all eight
spellings: two accepted, six refused by name with the enforced set in the sentence.

The stale-record half was found by a mutant that *survived*. Disabling the boundary's
in-transaction re-check changed nothing, because `start_prepared_task` refuses from the
preparation's own recorded reason first — so the re-check was untested and its job was
unclear. The job is a `Ready` preparation going stale between the prepare and the start:
an edge arrives after the report says ready. The check is now pinned by preparing a task
while it is genuinely free, recording an edge, reading the record back (still `ready`,
because a record is a record), and asserting the boundary re-asks anyway. That is the same
reason the boundary re-reads the stream, the route and the registration instead of
believing the preparation, and it is the one guarantee a report cannot give.

That half also caught a mistake in the test itself, twice: a prerequisite that had
*completed* satisfies every gate, so both the late edge and the first attempt at it
measured nothing. A gate is only a gate while the thing it names is unfinished.

Four mutants die on `a_gate_that_holds_a_task_back_is_named_by_the_report_the_projection_
and_the_boundary`: dropping `waiting_on.is_empty()` from `ready`; passing `Vec::new()`
where the store reads the gates; disabling the boundary's re-check; and reverting its
refusal to the old unit error. `depends_run.sh` walks the whole thing with first-class
commands and **no `raw` file**: the edge recorded, a provenance relation refused by name, a
malformed token refused by name, the projection, the report naming `demo/prereq` and the
state it is in, the boundary naming the same word, two controls, the gate opening when the
prerequisite lands, and the stale record refused at the boundary.

**Candidate, not a decision — the reap test's deadline race.** CI failed
`a_child_the_owner_cannot_reap_in_time_is_unreaped_and_not_clean` once on the 1.85.0 leg with
`Cancelled { group_signal: Sent }` where the test asserts `Unreaped { error: DeadlineExceeded }`,
and passed on re-run. Reading the code rather than the symptom: `cancel` checks `try_wait()` before
the deadline, so a zero-deadline `end` lands on whichever of two honest branches the child reaches
first, and `Drop` re-runs `cancel` with a 500 ms budget, so a timed-out session leaves no zombie
and no hang. The product behaviour is honest in both branches; the **test** asserts a branch of a
race the code does not promise. That is a flaky assertion, not a product defect, and it was left
alone.

**Found in the tenth pass — two streams cut from one base, and the second had no way
through.** The orphan sweep's standing note was that `StreamAction::{Collision, Reconcile}`
had `production = 0` while `StreamState::Collided` was unreachable. The ninth pass took the
four `TaskAction` orphans as its subject; this one took the stream half, because the sweep
can say a variant is unconstructed but not what an operator loses by that.

The subject was measured, not read. A transcript cut two streams from the same commit, ran
both to `completed`, integrated the first, and then asked the second what it could do. The
first probe ordered its own steps badly — it abandoned the stream before asking about
`advance` and `validate`, so those refusals were partly consequences of the probe — and
re-ordering it changed the picture: `advance-change-stream` **succeeded**, re-pointing the
stream at the Root's new head, and the next integrate failed with
`fast-forward integration refused: GitRefused`. So there was a second defect underneath the
first: the operation that looked like a way out was a way of renaming the collision. A
stream re-pointed at a Root its branch was never built on still cannot fast-forward, and
`GitRefused` is the whole of what git's answer survived as, because `GitExecutor::run`
discards stdout on a nonzero exit and this crate's `GitError` has no stderr.

The wedge was this. Two streams cut from one base both hold verified work and both expect to
be the Root's next commit. The first to integrate moves the Root. The second then has:
`integrate` refusing with `root checkout is not at the stream's recorded target`, naming
neither who moved the Root nor what to do; `release` refusing, because it needs an integrated
stream; `validate` refusing from `Collided`; and `advance` either refusing or, before this
pass, succeeding and making the next integrate fail. The only exit left was to abandon a
stream whose work had been verified, and throw it away.

So the collision is now a **state the Host records**, which is what makes the rest walkable:
`StreamState::Collided` refuses `Advance` and admits `Reconcile`, and the domain had said so
since the lifecycle was written. The integrate refusal names the stream that took the Root,
the commit it is at, and the work stranded on top of what it had recorded; the record of
*who* collided is read back on a second ask, so the answer does not depend on how many times
it was asked. The command id it is journaled under is derived from the Root's commit rather
than the caller's — a collision is the Host's own observation, and spending the caller's id
on it would hand a retry a receipt for an action it never asked for.

A collision with **nobody to collide with** is a different thing and is not recorded. When no
stream cut against that root accounts for the move — a person committed there, or the
checkout was reset — `Collision { other }` has no honest value to record, because the domain
refuses an `other` equal to this stream and inventing one would accuse a stream of something
it did not do. That case is told apart and points at `advance-change-stream`, which
re-points the stream at a Root nobody else is competing for, and is right for it.

Naming the holder needed a question the store could not answer: `Store::root_streams` reads
every stream cut against one root in one query, in id order, and is now pinned for order,
repeatability, and for *not* returning another project's streams — a refusal naming a stream
from another project would be worse than no refusal at all.

`reconcile-change-stream` gets the Root's commits into the stream's history, and it is a
**merge, not a rebase**, for a reason the first draft got wrong and the test caught. The
first draft rebased onto the Root and gated that on
`git merge-tree --write-tree --name-only -z`, asking whether the two tips merge without a
decision. Those are not the same question: a rebase replays the branch's commits one at a
time, so two tips can agree while an earlier commit still conflicts. The daemon test found
exactly that — after an operator resolved the conflicting file by hand so the *tips* agreed,
the preview said clean and the rebase still failed halfway. A merge of the Root into the
branch is one operation between the same two tips, which is precisely what `merge-tree`
computes, so the preview became the move asked in advance rather than an approximation of it.
It also stops rewriting every commit id the stream had already recorded, which is what the
operation was for: no id the store holds moves under a caller.

The preview needed a second thing, found by reading git's own frame rather than assuming it.
Git writes its `Auto-merging` and `CONFLICT` prose to **standard output**, interleaved with
the very paths the refusal has to name; a parser that trusted the lines would report
`CONFLICT (content): Merge conflict in shared/file.txt` as a filename, and the test caught it
doing so. `--no-messages` is what makes the frame readable, and a frame that is not a tree
followed by paths is not read as an answer at all — a frame with no NUL terminator is either
a foreign shape or one the bounded reader cut short, and reading a clean merge out of a
truncated one is the guess the rest of that crate refuses to make.

The refusals promise what they cost, which is the part a caller has to be able to trust.
Where both sides moved a path, the refusal names those paths and says `nothing was merged`;
the checkout's head is proved unchanged and no `MERGE_HEAD` is left behind. A checkout with
uncommitted work is refused *before* git is asked to start anything, with its changed and
untracked counts, because a merge over a dirty worktree fails after it has begun. A merge
that fails after the preview promised it would not is aborted, so the checkout is what the
caller had either way. And the ancestry is re-checked afterwards, because a merge that exited
zero and left the Root outside the branch's history would hand `integrate` the same failure
this operation exists to leave.

A caller who resolves the conflict by hand gets the same answer on the next ask, because the
question is asked of the repository and never of the request — that path is in the test, and
it is the reason a refusal costs nothing: the work is still exactly where it was.

Two of the guards in this pass are **not** held by a test, and are kept deliberately. The
`Integrated | Released` filter on the holder search survives mutation because no fixture can
produce a stream that is not integrated yet whose head is an ancestor of the Root's — a
`Validated` stream's head is on its own branch line, and the Root contains a head only once
it has landed, which is the state the filter is checking for. The post-merge ancestry proof
survives mutation because real git does not exit zero without merging. Both are defence in
depth against a git that behaves differently, and both are reported here rather than claimed.

**Found in the eleventh pass — a task that is not ready could not be held, and a held
task would not have been visible.** The tenth pass took the `StreamAction` half of the
orphan sweep, which left `TaskAction::{Queue, Assign, Block, Unblock}` at `production = 0`.
This one took `Block` and `Unblock`, and the question the sweep cannot answer is what an
operator loses by a variant nobody constructs.

Measured, not read. A transcript created two tasks and asked what can be done with one
that is not ready:

```
  hold-task                    no such command
  block-task                   no such command
  pause-task                   no such command
  defer-task                   no such command
  the only way to hold it      create-task dependencies on a task that never finishes

  and the task is still ready, with nothing on it saying why:
      state: ready
      keys a caller can read: dispatch, history, id, project_id, revision, role_id, root_id, state, stream_id, task_contract
```

So the product offered exactly two moves — create it and let it run, or never create it
— and one pretending to be a third. The pretending is the interesting part, because it
works right up to the reporting: a dependency on a task that never finishes does hold
this one, and the store reports `dependency_unresolved` naming a task that has nothing
to do with the real reason. So the only way to say "not until the key is provisioned"
made the store say something else, and the task itself carried nothing but `state: ready`.

The second half is the compounding one, and it was found by looking rather than by
guessing. The scheduling projection's query reads
`json_extract(body,'$.state') IN ('ready','assigned')` — so a held task would have been
absent from **both** its lists. That is not "nothing is blocking this"; it is silence,
and silence about a task somebody is deliberately waiting on reads as a task that is not
waiting. This is the ninth pass's own lesson (`a report that says ready is worse than no
report`) applied to a new gate, and it would have been introduced by fixing the first
half of this one.

Two things were checked and found *not* to be defects, which is why they are recorded
here rather than fixed. A running task's absence from the projection is by its stated
contract — `blocked` means "start candidates that cannot run", and a running task is
neither, with `dag.progress` carrying the in-flight count. And `BlockedReason`'s
`NotConsidered` doc promises `progress.partial` is set whenever it appears; that field
does exist, on the graph report, and is set from the same bounded answer the reason comes
from. Both were nearly written up as findings.

So a hold is now `block-task <task_id> <reason>` and its end is
`unblock-task <task_id> <reason>`, with the reason required at the command line because
a hold the projection cannot explain is a hold that has to be lifted to find out what it
was for. The reason is the operator's own sentence rather than a category, and it is
read back out of the task's command history by `Task::blocked_by`, which is the only
place the domain keeps it and therefore the only place it can be read from.

Three surfaces now answer the same question with the same words: the projection reports
`operator_held` with the sentence in a new `BlockedTask::detail`; the dispatch report
records a `hold` step; and a second `block-task` on a held task repeats the reason it is
held for. A hold is asked about *before* every derived blocker and wins when it applies,
because naming a lease the operator cannot act on would hide the sentence they wrote, and
naming a dependency for a task whose hold they lifted is a category error.

**The dispatch report was the defect the transcript found, and it was found after the
fix rather than before it.** `prepare-dispatch` on a held task returned
`outcome: refused` with `scheduling: {schedulable: false}`, `dependencies: {waiting_on: []}`,
`worktree: {startable: true, state: "active"}`, `routing: {resolved: "engineer"}` and
`provider: {validated: true}` — every fact true, the refusal right, and nothing anywhere
saying a person was holding the task. An operator reading that before dispatching goes
looking for a stream problem, a lease or a missing route. `CompositionStep::Hold` now
carries the reason, read off the task rather than passed into `compose`, because a caller
that had to supply it could supply a reason for a task that is not held.

Eight dying mutants hold this pass: the projection's SQL dropping `blocked` again; the
hold ceasing to win over a derived blocker; the report's `hold` step not being recorded;
the report recording that a hold exists but not its reason; a hold reason outliving the
hold; the already-held refusal ceasing to repeat the reason; the hold's empty-reason
refusal dropped; and the release's empty-reason refusal dropped.

The four `TaskAction` orphans are now two: `Queue` and `Assign` remain, and they are a
different question. They are the canonical scheduler's own states — `Queued` is "the Host
has admitted this to its durable queue" and `Assigned` is "the Role is explicitly bound
before dispatch" — so producing them means building the queue that admits them, which is
not a missing command but a missing component. Worth saying plainly rather than
discovering it as a twelfth-pass surprise.

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
