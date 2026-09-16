# Linux Tauri shell proof — #38

Status: experimental fixture, **not shell selection**. The integrating engineer authorized execution after independently reviewed foundation PR #473 merged at `a15368c1744fdff3c186dda11f24714d738c7efe` with stable/MSRV/roadmap checks passing. This isolated spike does not alter the production workspace. Baseline: merged #472 and ADR-0001. It is a deliberately partial workload; an Xvfb run cannot pass the complete #38 contract, which is committed as data in [`spike-contracts.json`](../architecture/spike-contracts.json) with its predeclared ceilings, stop conditions and cleanup. That contract does not state its own bar, and does not choose it either: the [decision ledger's](../architecture/decisions.json) record for the shell choice names the "Proof contract and stop conditions" section of [ADR-0001](../architecture/adr-0001-technology-direction.md) as the bar its proof must answer, and the contract answers every clause of that section with an obligation its runs must exercise, with its own method or ceilings, or with the issue that owns a clause belonging to #38's wider acceptance — each answer carrying that clause's own words, read back against ADR-0001 rather than trusted — which is where the checks listed below as pending are owned. Nothing here is that run: the Wayland run recorded below is the first real one measured against the contract, and it is a partial run set that settles nothing.

## Reproduce

From `spikes/linux-shell`: `npm ci --ignore-scripts`, `npm run build`, then `cargo build --locked -j 4 --manifest-path src-tauri/Cargo.toml`. Run native authorization unit tests with `cargo test --locked -j 4 --manifest-path src-tauri/Cargo.toml`. After the prerequisite gate is established, `SYMBIOTE_PROOF_AUTHORIZED=1 python3 run-proof.py` creates an isolated Xvfb server, runs this app for 20 seconds, captures process-tree RSS samples and an optional ImageMagick screenshot, checks observed survivor processes, and removes the server. It never uses the user's desktop display. Artifacts are under ignored `spikes/linux-shell/artifacts/`. `python3 -m unittest test_run_proof` runs the rules that decide what a Wayland run may publish, and needs neither a session nor a binary. A run that publishes a dossier records the revision it was built from, so the clean build it cites is logged with `git rev-parse HEAD` in the same `set -x` recipe.

Pinned direct components: Tauri 2.11.5, tauri-build 2.6.3, portable-pty 0.9.0, Tauri JavaScript API 2.11.1, React 19.2.8, TypeScript 7.0.2, Vite 8.2.2, Monaco 0.56.0. Cargo/npm lockfiles retain the transitive resolution. Monaco's DOMPurify dependency is explicitly overridden to 3.4.15 because its upstream pin reports security advisories; the resulting npm audit reports zero vulnerabilities at preparation time. No system/global package or user application changes. Local prerequisite discovery: GTK 3.24.52 and WebKitGTK 2.52.6 available on the Linux host.

Primary references inspected on 2026-09-08: [Tauri native child WebView API](https://docs.rs/tauri/2.11.5/tauri/webview/struct.WebviewBuilder.html), [Tauri capability isolation](https://v2.tauri.app/security/capabilities/), [portable-pty](https://docs.rs/portable-pty/0.9.0/portable_pty/). Multi-WebView uses Tauri's explicitly **unstable** feature; that is an architecture risk, not an accepted stable production dependency.

## Prepared workload and acceptance limits

- Four concurrent synthetic 10 Hz UI streams retain 40 lines each. These are **not agent streams** and do not prove provider/runtime behavior.
- Three real native PTYs run independent Python processes with tty detection and 4 Hz output. Each output ring is capped at 16 KiB; the UI displays one selected terminal. No interactive shell/input forwarding or arbitrary descendant process-tree cleanup is claimed. Stop kills and reaps these three single-process workloads.
- A real Monaco diff editor is loaded lazily only in the workbench; disposing/switching destroys its models. Two in-memory Project drafts demonstrate local switching only. No filesystem source changes, durable recovery, Host reconnect or concurrent controller proof.
- The untrusted Preview is a separate incognito native child WebView on an ephemeral loopback origin, with navigation restricted to that origin and new windows denied. It has no capability assignment. Every custom native command checks both the native WebView label and trusted `tauri://localhost` origin; Preview attempts `snapshot` and `stop_ptys` and reports denied/absent/unexpected outcomes. Workbench and Preview do not share a privileged iframe. An authenticated narrow design bridge and source edits are absent and remain required.
- Floating Lead controls are a third native child WebView overlapping Preview. They contain a focus-test input; no voice or agent command behavior is implemented. Linux requires an explicit GTK Fixed container because Tauri's default VBox stacks the views. Fixed rectangles leave resize/scaling correctness unpassed; one static overlap observation is not general composition/focus proof.
- Instrumentation records startup/readiness, native denials, Preview probe reports, PTY start/stop, app exit, and process-tree RSS. RSS sum double-counts shared pages and excludes components not present; no whole-product resource threshold is claimed.

Hard fixture failures: build failure; native command unexpectedly allowed from Preview; failure to render the prepared surfaces; nonzero app exit; observed surviving PTYs after clean exit. The runner asserts lifecycle/readiness and two denied Preview probes only; a zero exit does not automatically certify rendering or interaction. Such failures block declaring this fixture passed. No numeric production performance gate is invented after measurement.

## Observed runs (2026-09-08 UTC)

First run `artifacts/20260908T032747Z`: exit 0 and both Preview commands denied, but the screenshot revealed a **failed** layout: default native VBox stacked all three WebViews, clipping editor/terminals and preventing floating controls. Peak summed app-tree RSS was 1,447,024 KiB. This failed composition evidence is retained.

After explicit GTK Fixed layout, run `artifacts/20260908T033012Z` lasted 21.003 seconds (20-second requested app lifetime), Xvfb 1440×960 and app 1400×900. Exit 0; Preview self-reported denial of both `snapshot` and `stop_ptys`; three native PTYs started and were reaped; zero sampled survivors before or after cleanup. Screenshot inspection shows four synthetic streams, Monaco diff, separate Preview, overlapping Lead input, three PTY tabs, and tty output. Interaction, keyboard focus and project switching were not exercised in this run. Native denial counter is zero; framework IPC rejection is inferred, not independently proven. The separate unit test verifies the custom label+origin defense.

Peak **summed RSS**, not physical RAM/PSS, was 1,336,624 KiB at 1.680 seconds: shell 220,060 KiB, three PTY processes 40,684 KiB, WebKit network/renderers 1,075,880 KiB. Individual WebKit renderer-to-label attribution remains unknown. Xvfb is separate instrumentation and was not included in that peak; subsequent runner samples report it separately. Mesa logged lack of accelerated DRI3 on Xvfb. These debug/software-display observations cannot pass a production memory/performance gate. Raw `app.log`, `process-tree.json`, `cleanup.json`, `xvfb.log`, and `x11.png` remain in the local artifact directories above.

Evidence qualification: references above to Preview commands being denied mean **Preview self-reported denial**, received through an unauthenticated report endpoint. The historical raw `preview_probes_denied` field means only those strings were received. The custom handler was not observed rejecting these attempts; framework rejection is inferred, not independently proven. The label/origin unit test is separate evidence. This run does not certify the Preview security requirement.

Interactive run `20260908T033453Z` used `--interact` on the private Xvfb only. Independent screenshot inspection confirms typed Preview and Lead input values, a Beta draft, and the original Alpha draft restored after returning to Alpha. This proves bounded pointer/input switching and in-memory draft separation at the fixed geometry. The stop screenshot shows terminal selection but does not itself prove stop completion; native logs and process samples are the lifecycle evidence. No IME, keyboard-only navigation, accessibility or durable restoration is claimed. The run exited 0 after 36.108 seconds, reported no observed survivors and peaked at 1,225,752 KiB summed RSS. Selected raw artifacts, including the failed layout, are committed under `evidence/linux-shell/`.

Reviewer cleanup correction: the runner retains PID birth ticks and records surviving observed descendants before cleanup; its `finally` block kills only matching observed identities even if the parent already exited, then writes separate cleanup results. Sampling cannot account for a short-lived/unobserved descendant; this is not a complete supervision implementation. PTY reader acquisition now occurs before child spawn, removing a child-leak failure path. Explicit false-green avoidance: no missing probe report is counted as present and no surviving descendant is counted as clean.

## Wayland session run (2026-09-16 UTC)

The runner supports two private sessions: the Xvfb path above, and `--session
wayland`, which starts its own `kwin_wayland --virtual` compositor in a private
`XDG_RUNTIME_DIR` and runs the app with no `DISPLAY` at all. Both leave the user's
session, display and packages alone. The Wayland path samples the app's process
tree every 100 ms with PSS per member and per component class, cancels a live run,
and writes the run dossier and the result artifact the contract points at
([`results/desktop-shell.json`](results/desktop-shell.json)) from what it observed;
the rules that decide what it may publish are held by `test_run_proof.py`, which
the fixture's CI job runs beside `py_compile`.

Platform finding, recorded with the run as the contract asks: with this host's
NVIDIA/glvnd EGL the client aborts against the virtual backend with
`wp_linux_drm_syncobj_surface_v1` explicit-sync errors, so the session pins Mesa's
EGL ICD (`/usr/share/glvnd/egl_vendor.d/50_mesa.json`) and records the renderer it
actually used, in `session.json`. That is a session compromise, not a change to the
app: the committed contract's applicability names the *reference compositor*, and
this run is on a virtual one.

What the run observed on 2026-09-16, built from `988c37af` in a clean target
directory outside the tree (kwin 6.7.5 virtual session at 1440×960, Mesa Intel(R)
Graphics (RPL-P), 16 logical cores, 62 GiB RAM, NVMe storage), with the evidence
under `results/desktop-shell/linux-wayland-on-the-reference-compositor/`. The dossier
also records the contract's fingerprint (`89a55836…`) — the crate's SHA-256 of the
contract's own canonical JSON, which ties these figures to the thresholds they were
measured against, and not the SHA-256 of the contract document — and the revision
the run was built from; the command line that supplied the fingerprint is not part
of the record. Each figure is re-checkable from the artifact the run cites for it:

- **`cold_start_to_first_frame_seconds` 0.353** of a 3.0 ceiling: the first
  `wl_surface.commit()` after the client attached a buffer, timed from its first
  request in `app-1.log`, which went on to commit 452 frames, the last at 20.302 s.
- **`clean_locked_build_seconds` 116.979** of a 900 ceiling: `npm run build` and
  `cargo build --locked --release` from an empty target directory, in a log
  captured with `set -x` that names those commands and the revision it read (
  `Finished \`release\` profile [optimized] target(s) in 1m 55s`).
- **`workload_process_tree_pss_mib` 485.490** of a 2560 ceiling and
  **`unattributed_process_tree_memory_percent` 1.525** of a 5.0 ceiling: the peak
  of 251 samples of the app tree's summed PSS (shell 90.8 MiB, three terminals
  21.7 MiB, WebKit helpers 365.5 MiB, unattributed 7.4 MiB). The peak fell 1.138 s
  into a 25.063 s window, which is a startup peak rather than a steady-state
  figure; the run records where it fell with the figure instead of leaving a
  reader to guess, and 11 tree members were alive at it.
- **`orphaned_processes_after_cancel` 0** and
  **`orphaned_listening_ports_after_cancel` 0** of 0 ceilings: SIGTERM to the live
  tree 25 s into the second run left no observed member and no listening port, and
  the driver's own cleanup confirmed it before the compositor was torn down. The
  survivor count is read from the process state, so it means nothing is left
  running rather than nothing is left listed, and the entries that had exited but
  were not yet reaped are named beside it (`unreaped_observed_pids`, empty here).

These are a re-run of the same declared run set, taken because `session.json` had
to stop pointing at paths the run removes: the stop condition, the five
measurements reported unknown with their reasons, the three recorded limitations
and the three obligations `exercised` are unchanged, while every figure the run
observed is this run's own.

What it could not observe, reported unknown rather than met: idle-state PSS (this
fixture has no idle state), workbench readiness (its `PROOF_READY` marker precedes
the first frame and every WebKit helper, so it is not a workbench-ready signal),
input starvation (this driver issues no input on Wayland), journal loss across a
crash (the fixture has no canonical store) and installer size (nothing here
packages an installer). No frames were captured in this session: the X11 path's
screenshot has no equivalent in the Wayland runner, so the rendering evidence is
the client's own protocol log and the app's logs.

The result attests only what the fixture's own log shows. `exercised` is derived
from the app's markers — three `PROOF_PTY_START`, one `PROOF_READY
preview_origin=`, two `PROOF_PREVIEW_REPORT` denials — so four concurrent agent
streams are absent from it even though the fixture starts four synthetic ones. The
runner refuses to publish a result whose platform the contract does not apply to,
whose stop condition the contract does not declare, which leaves a predeclared
measurement neither observed nor named with its reason, or which attests an
obligation no marker supports; the platforms this run did not exercise are derived
from the runs the dossier holds rather than typed; and the artifact it wrote was
read back by the ledger's own map, which reported no refusals for it.

A dossier is one contract's, and this driver runs one platform at a time, so a run
merges: another platform's recorded runs are kept exactly as they stand, this
platform's own entries are replaced by what the invocation observed, and the
untested list is the contract's applicability minus the platforms the dossier
actually holds — a platform whose run is recorded cannot be left declared untested.
Each platform publishes its evidence into its own directory under the artifact's,
so a second platform's app and session logs cannot overwrite the first's. A dossier
of another contract, or one measured against other thresholds, is refused rather
than merged into. The session record names each of its paths the run does not keep
— the private runtime directory it removes, the binary it built outside the tree,
the build log that goes with that target directory — beside what outlives it: the
binary's hash, and the committed copy of the build log with the same bytes. The
revision a run records has to be the tree it ran in — clean, at `HEAD` — and the
recipe it cites has to have logged the same revision, because a
build log that names only its commands cannot be tied to a tree.

Nothing here settles #38. Three of the contract's four platforms are declared
untested in the result artifact, and the workload exercised is the prepared
fixture, not the product's agent streams, editor source mapping, cancellation path
or canonical store. Six of the contract's eleven predeclared measurements were
taken, and every run entry would have to answer all eleven for a choice to settle.

Pending full #38 acceptance: Wayland and real X11 compositor behavior; Windows/macOS; real agent streams; terminal interaction; all geometry, IME, accessibility, focus/stacking and scaling checks; design-bridge authentication/DOM-to-source mapping; crash recovery/replay/reconnect; simultaneous controllers/headless Host; watcher behavior; menus/updater/signing/installers; suspension/cancellation across services; release/idle/cold-start CPU/PSS measurements and representative database/analyzer/provider workloads. A screenshot alone cannot prove these properties. No shell decision is authorized by this partial spike.
