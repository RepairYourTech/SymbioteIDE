# Preview isolation — live verification on the kwin Wayland session

Owner: #54. This record closes the live verification that PR #524's Preview surface is isolation-real: the REAL binary (`target/debug/symbiote-desktop`) was launched on the operator's live KDE/kwin **Wayland** session, a run was driven through the UI by AT-SPI, the Preview window was opened for the run's output, and the isolation properties were verified **from inside the live page** — the strongest available proof, because the AT-SPI text interface reads the page's live DOM text: if the hostile script had executed, its observable marker would appear in this capture; it does not. Companion artifacts: [`evidence/preview-isolation/`](evidence/preview-isolation/) (`preview-window-atspi.txt`, `main-window-atspi.txt`, `atspi_drive.py`).

## The hostile fixture report (new, permanent)

The fixture transport's report is now **hostile by default** (`symbiote_workflow::demo::FIXTURE_REPORT`): the bounded-change sentence followed by `<script>document.body.appendChild(document.createTextNode("WORKER_SCRIPT_EXECUTED"))</script><img src=x onerror="document.title='WORKER_IMG_ONERROR'"><b>bold markup</b>`. The script's payload is DOM-observable: **if it ever executes anywhere, the marker text `WORKER_SCRIPT_EXECUTED` is appended as its own node** — distinguishable from the same string appearing inside the escaped literal. Every rendering surface (desktop UI log, Preview window, journal evidence) therefore permanently proves inert rendering on every run, not just on a staged one.

## What the live run captured (2026-09-10, head `e545e51e` + this record)

1. **The flow, driven by AT-SPI on the real UI** (`evidence/preview-isolation/atspi_drive.py`): begin session (the app spawned the real `symbioted`) → start demo (dispatch `disp_staffing-task`) → read journal → finish demo → **Preview run output**.
2. **The hostile report flowed through the whole stack as data**: the main window's log shows `finish-demo: state=completion_requested report="implemented the bounded change; … <script>document.body.appendChild(…WORKER_SCRIPT_EXECUTED…)</script> …"` (React escapes it; `main-window-atspi.txt`), and the completion evidence carries it (Worker completion remains evidence; Host verification and independent review stay the gates).
3. **The Preview window is its own frame** (`Symbiote — Preview (untrusted worker output)`), rendering `Run disp_staffing-task`, the isolation statement, and — in the Report section — the hostile markup **as literal text** (`preview-window-atspi.txt`).
4. **Inertness discriminator**: the marker `WORKER_SCRIPT_EXECUTED` appears **exactly once** — inside the escaped literal. The checker (inline in this record's method) fails if execution ever appended a second, standalone marker. The `<img onerror>` payload is likewise visible only as text. *Note: the AT-SPI text view decodes HTML entities, so the captured text shows `<script>` where the served document has `&lt;script&gt;` — the escaped source form is pinned by the unit tests merged in #524 (`worker_content_renders_as_inert_text`).*
5. **The ACL rejection, live and in-context** (the #524 review's explicitly deferred verification): the Preview window's isolation self-check — an owner-injected probe, evaluated in the preview's own context after page load — attempted the read-only owner command `journal_position` **from the preview window** and the app ACL rejected it. The page records the platform's own diagnostic: `Isolation self-check [probe probe-3049777-403963934]: OK: app command rejected by the app ACL (journal_position not allowed on window "preview", webview "preview", URL: local allowed on: [windows: "main", URL: local] referenced by: capability: default, permission: allow-journal-position)`. The probe is a constant of the binary bound to an owner-generated nonce (rendered in the document heading, unpredictable to worker content, so a report-embedded fake verdict line cannot impersonate the self-check); it runs on every page load, so the surface re-attests its isolation on every open. The verdict is evidenced in the page body (committed artifact); the document-title mutation is not captured and is not relied on.
6. **Graceful stop**: the UI `Stop` button — daemon gone (`pgrep -ax symbioted` empty).

## Honest boundary — what this does NOT claim

- **No live model turns**: the run's model turn is the labeled fixture transport. **A live native OpenAI Responses API turn and a live Codex 0.118.0 App Server turn still require explicit user authorization for credentials and billing** — nothing was spent here (and the fixture report is now deliberately hostile markup, which the preview renders inert).
- **No pixel capture of the Preview window**: the window sat behind the operator's active work, and focus-stealing was avoided deliberately. The isolation proof does not need pixels — the AT-SPI capture reads the live DOM text, which is stronger than pixels for this claim (an executed script's DOM marker would have been captured). Rendering itself is already evidenced by PR #523's committed screenshots.
- The probe's verdict is the platform's own rejection diagnostic, captured from the live page; the same property is separately CI-pinned by #524's artifact test (generated ACL manifest + resolved capabilities).
- One machine, one kwin/Wayland session; X11/other-desktop evidence and Windows/macOS remain pending.

## Reproduce

From the repository root (workspace binaries built):

```sh
mkdir -p /tmp/symbiote-desktop-demo/{state,worktrees}
git init -q -b main /tmp/symbiote-desktop-demo/repo
SYMBIOTE_BIN_DIR="$PWD/target/debug" ./target/debug/symbiote-desktop
# drive via docs/proofs/evidence/preview-isolation/atspi_drive.py:
#   click "Begin session" -> "Describe task & start dispatch" ->
#   "Read evidence trail" -> "Run & read evidence" -> "Preview run output"
# then dump the preview frame's text and run the committed checker:
#   python3 docs/proofs/evidence/preview-isolation/atspi_drive.py marker
# which asserts:
#   - WORKER_SCRIPT_EXECUTED appears exactly once (inside the escaped literal)
#   - "Isolation self-check: OK: app command rejected by the app ACL" is present
```
