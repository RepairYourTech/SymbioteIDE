# Tauri desktop shell — real Wayland acceptance evidence

Owner: #54 (the desktop workflow this shell drives); closes the "real Tauri acceptance on Wayland/X11" item of the first-release definition of done **for the Wayland part**. This record is captured from the REAL binary (`target/debug/symbiote-desktop`, Tauri 2 + WebKitGTK) launched on the operator's live KDE/kwin **Wayland** session (`XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY=wayland-0`), not from a headless test. Artifacts live in [`evidence/tauri-wayland/`](evidence/tauri-wayland/). Nothing here is spent: the model turn in the driven flow is the operator's explicitly labeled fixture transport, and the UI states the live-turn gate in-product (visible in the screenshots).

## What was captured (2026-09-10, head `020c0cdd` + this record)

| # | Evidence | Artifact / observation |
|---|----------|------------------------|
| 1 | **Launch on Wayland** | `SYMBIOTE_BIN_DIR=target/debug ./target/debug/symbiote-desktop` — the window maps on the kwin Wayland session; the GTK frame exposes itself as AT-SPI application `symbiote-desktop` with frame caption `Symbiote`. |
| 2 | **Accessibility tree** | WebKitGTK exposes the whole React UI through AT-SPI (`ATSPI_DOCUMENT_WEB`): the `Symbiote` heading, the live-turn fixture notice paragraph, three labeled entries carrying their values (`State directory`, `Worktree base`, `Repository`), and every button (`Begin session`, `Stop`, `Describe task & start dispatch`, `Read evidence trail`, `Run & read evidence`, `Journal position`). Dumped with `docs/proofs/evidence/tauri-wayland/atspi_drive.py` (Atspi over the session's a11y bus). |
| 3 | **Focus** | KWin scripting (`evidence/tauri-wayland/focus-symbiote.js`, loaded via `org.kde.kwin.Scripting.loadScript` + `.start`) activates the window; the AT-SPI frame then reports `STATE_ACTIVE` + `STATE_SHOWING`. |
| 4 | **Composition/rendering** | `spectacle -a` captures the focused window fully rendered — title bar, styled UI, live log text (`evidence/tauri-wayland/01-focused-window.png`). Not a blank/composited-black window: WebKitGTK composition works under the Wayland session. |
| 5 | **Whole demo driven through the a11y tree** | Every step was performed by AT-SPI button activation on the real UI: `begin: started` (the app spawned the real `symbioted` — separate process, verified via `pgrep -ax symbioted`, with its SQLite store and operator config under the UI-provided state directory) → `start-demo: dispatch disp_staffing-task` (repository opened, objective/task composed, dispatch STARTED) → `read-journal: 12` (evidence trail to cursor 12) → `finish-demo: state=completion_requested report="implemented the bounded change; produced.txt written by the sandboxed tool" produced=/tmp/symbiote-desktop-demo/worktrees/staffing-demo/st-c3664a298fb65755`. Final state: `evidence/tauri-wayland/02-demo-complete.png`. The daemon-spawn observation was made live (mechanism confirmed in `controller.rs` `begin_generation`); the log lines themselves are in the committed screenshot. |
| 6 | **Real worktree output** | The run's sandboxed shell tool really produced `produced.txt` (`worker-output`) inside the derived reserved worktree named in the UI's own log — filesystem-observed live (`ls` + `cat` during the session, not captured as an artifact); the produced path matches the derived worktree scheme (`base/<project>/st-<digest>`), and the worktree path itself is visible in the committed screenshot's log. |
| 7 | **Graceful stop** | The UI `Stop` button: the app's log shows `stop: stopped`, and the daemon process is gone (`pgrep -ax symbioted` empty). *Observed live in the session; not captured as an artifact — 02-demo-complete.png is the pre-stop state. The stop path is mechanism-confirmed by the code (lib.rs `stop_session` → controller `stop()`; the log line renders from `runAction`).* |

## Method

The driver is ordinary AT-SPI — no screenshot matching, no synthetic global input:

```sh
python3 docs/proofs/evidence/tauri-wayland/atspi_drive.py click "Begin session"
python3 docs/proofs/evidence/tauri-wayland/atspi_drive.py log
```

(`click` resolves a button by accessible role + name and invokes its AT-SPI action; `log` walks the tree and reads every text-bearing node via `Atspi.Text.get_text`.) Screenshots use `spectacle -a -b -n -o <file>` against the focused window. Focus uses the KWin scripting D-Bus interface (`loadScript` → `start` → `unloadScript`).

## Honest boundary — what this does NOT claim

- **X11-only sessions, other compositors, other desktops**: this evidence is one kwin/Wayland session on one machine. An XWayland/X11-run instance was not separately exercised; Windows/macOS remain explicitly pending. The launch used the session's own environment; no minimal-compositor (weston/headless) run was attempted.
- **Secure Preview** is not part of this shell yet — there is no Preview surface in this UI to accept, so no Preview isolation evidence exists (separate definition-of-done item).
- **Whole-process measurements** (memory/composition cost under load) were not taken; this record is functional acceptance, not a performance one.
- The app did not exit on `SIGTERM` in one observation (it required SIGKILL after the session was already stopped). The graceful path — the UI `Stop` button — worked and left no daemon; the controller's `Drop` guard (PR #520) bounds any non-graceful path. SIGTERM handling of the Tauri main loop is noted as a follow-up observation, not a defect proven here.
- The driven flow's model turn is the labeled fixture transport. **A live native OpenAI Responses API turn and a live Codex 0.118.0 App Server turn still require explicit user authorization for credentials and billing** — nothing was spent; the UI itself states this gate (visible in both screenshots).

## Reproduce

From the repository root (workspace binaries built):

```sh
mkdir -p /tmp/symbiote-desktop-demo/{state,worktrees}
git init -q -b main /tmp/symbiote-desktop-demo/repo
SYMBIOTE_BIN_DIR="$PWD/target/debug" ./target/debug/symbiote-desktop
# then drive via docs/proofs/evidence/tauri-wayland/atspi_drive.py
```
