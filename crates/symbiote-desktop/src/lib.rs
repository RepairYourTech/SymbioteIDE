//! The Symbiote desktop shell: Tauri commands over the headless
//! [`controller`]. The commands are deliberately thin — every behavior
//! they expose is the controller's, which is proven against a real
//! daemon in headless tests. The UI never sees secrets, daemon state
//! internals, or anything but the sequencer's results.
//!
//! The Preview window is the untrusted surface: it renders worker
//! output as inert text and carries NO Tauri IPC (it is named in no
//! capability file), so worker content can never invoke the owner's
//! commands or reach the local-owner policy.
pub mod controller;

use controller::{DesktopController, DesktopError, DesktopPaths};
use std::{path::PathBuf, sync::Mutex};
use tauri::Manager;

/// The managed controller. `None` until the operator begins a session.
struct Session(Mutex<Option<DesktopController>>);

/// The Preview window's document, built by [`preview_document`] and
/// served through the `preview://` custom protocol. Worker output is
/// UNTRUSTED: the document is fully HTML-escaped, script-free, and
/// bound to `default-src 'none'`, and the window has no Tauri IPC.
struct PreviewDocument(Mutex<String>);

/// Poison-proof Preview lock: the document is a self-contained string
/// rebuilt wholesale on every open, so recovering from a poison cannot
/// surface a half-written value.
fn preview_lock(document: &PreviewDocument) -> std::sync::MutexGuard<'_, String> {
    document
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Builds the Preview document for a finished run. Every dynamic value
/// is HTML-escaped and there is no script: worker content can only ever
/// render as text, whatever it contains.
fn preview_document(dispatch_id: &str, report: &str, worktree: &str, files: &[String]) -> String {
    fn escape(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
    let mut files_html = String::new();
    for file in files {
        files_html.push_str("<li>");
        files_html.push_str(&escape(file));
        files_html.push_str("</li>");
    }
    let mut document = String::new();
    document.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    document.push_str("<meta charset=\"utf-8\">\n");
    document.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; form-action 'none'; base-uri 'none'\">\n");
    document.push_str("<title>Symbiote — Preview (untrusted worker output)</title>\n");
    document.push_str("<style>body{font-family:sans-serif;margin:1rem;background:#14151a;color:#e8e8e8}h1{font-size:1.1rem}pre{white-space:pre-wrap;border:1px solid #333;padding:.5rem}</style>\n");
    document.push_str("</head>\n<body>\n");
    document.push_str("<h1>Preview — untrusted worker output</h1>\n");
    document.push_str("<p>Run <code>");
    document.push_str(&escape(dispatch_id));
    document.push_str("</code></p>\n");
    document.push_str("<p>This surface renders worker output as inert text. It has no application commands: worker content can never invoke the owner's operations, and no script or network fetch is permitted here.</p>\n");
    document.push_str("<h2>Report</h2>\n<pre>");
    document.push_str(&escape(report));
    document.push_str("</pre>\n<h2>Worktree</h2>\n<pre>");
    document.push_str(&escape(worktree));
    document.push_str("</pre>\n<h2>Files produced (");
    document.push_str(&files.len().to_string());
    document.push_str(")</h2>\n<ul>\n");
    document.push_str(&files_html);
    document.push_str("</ul>\n</body>\n</html>\n");
    document
}

/// The isolation self-attestation: an owner-owned constant, evaluated in
/// the Preview window's own context after every page load. It attempts
/// the read-only owner command `journal_position` FROM THE PREVIEW and
/// reports the platform's verdict in the window title and the page body:
/// the app ACL must REJECT it (the preview window is named in no
/// capability). If the IPC plumbing is absent entirely, that is reported
/// too. Worker content never reaches this eval — it is a constant of
/// this binary — and the worker's own hostile markup is inert by
/// construction (preview_document).
const ISOLATION_PROBE: &str = r#"(function () {
    var report = function (verdict) {
        document.title = "Preview " + verdict;
        var line = document.createElement("p");
        line.textContent = "Isolation self-check: " + verdict;
        document.body.appendChild(line);
    };
    try {
        if (typeof window.__TAURI_INTERNALS__ === "undefined") {
            report("OK: no IPC plumbing is present at all");
            return;
        }
        window.__TAURI_INTERNALS__
            .invoke("journal_position", {})
            .then(function () {
                report("FAILED: an app command was allowed from the preview window");
            })
            .catch(function (error) {
                report("OK: app command rejected by the app ACL (" + (error && error.message ? error.message : error) + ")");
            });
    } catch (error) {
        report("OK: no IPC plumbing is reachable (" + error + ")");
    }
})();"#;

/// Opens (or refreshes) the Preview window for the LAST finished run.
/// The main window is the owner surface; this command hands the
/// Preview nothing but worker output, and the output is escaped into
/// an inert document by [`preview_document`].
#[tauri::command]
fn open_preview(
    app: tauri::AppHandle,
    dispatch_id: String,
    report: String,
    worktree: String,
    files: Vec<String>,
) -> Result<String, String> {
    let document = preview_document(&dispatch_id, &report, &worktree, &files);
    let state = app.state::<PreviewDocument>();
    *preview_lock(&state) = document;
    if let Some(window) = app.get_webview_window("preview") {
        // The document is served from managed state: a reload picks the
        // new content up. The reload navigation is owner-initiated; the
        // document's CSP bound still applies to whatever loads.
        window
            .eval("location.reload()")
            .map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok("preview updated".into());
    }
    let url = tauri::WebviewUrl::CustomProtocol(
        tauri::Url::parse("preview://localhost/document").map_err(|error| error.to_string())?,
    );
    // Two rapid opens can race past the refresh check; the loser falls
    // back to the refresh path instead of failing the click.
    if tauri::webview::WebviewWindowBuilder::new(&app, "preview", url)
        .title("Symbiote — Preview (untrusted worker output)")
        .inner_size(720.0, 520.0)
        .on_page_load(|window, payload| {
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = window.eval(ISOLATION_PROBE);
            }
        })
        .build()
        .is_err()
    {
        if let Some(window) = app.get_webview_window("preview") {
            window
                .eval("location.reload()")
                .map_err(|error| error.to_string())?;
            window.set_focus().map_err(|error| error.to_string())?;
            return Ok("preview updated".into());
        }
        return Err("preview window could not be opened".into());
    }
    Ok("preview opened".into())
}

/// Poison-proof session lock: a panicking command must not brick the
/// whole session. Recovering the inner value after a poison is sound
/// here — the guarded value is the controller itself, whose Drop still
/// bounds the daemon and whose `begin_generation` is idempotent across
/// the mid-operation states this crate's commands can leave (it cannot
/// detect an externally dead daemon; that refusal is pre-existing and
/// unchanged). Every command takes the session through this helper.
fn lock_session(session: &Session) -> std::sync::MutexGuard<'_, Option<DesktopController>> {
    session
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Resolves the daemon binaries: `SYMBIOTE_BIN_DIR` when set (a bundled
/// release or a test), otherwise the workspace target directory relative
/// to this crate (development). Bundled sidecar packaging is future scope.
fn resolve_bin_dir() -> Result<PathBuf, DesktopError> {
    if let Ok(dir) = std::env::var("SYMBIOTE_BIN_DIR") {
        return Ok(PathBuf::from(dir));
    }
    for ancestor in PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .skip(1)
    {
        let target = ancestor.join("target/debug");
        if target.join("symbioted").is_file() {
            return Ok(target);
        }
    }
    Err(DesktopError::MissingBinary(
        "symbioted (build the workspace)".into(),
    ))
}

/// Begins (or replaces) the desktop session: prepares the private
/// environment and spawns the daemon generation. Safe to call again after
/// a crash — the persisted journal positions make the restart identical
/// to a fresh boot.
#[tauri::command]
fn begin_session(
    state: tauri::State<Session>,
    state_dir: String,
    reservation_base: String,
    repository: String,
) -> Result<String, String> {
    let bin_dir = resolve_bin_dir().map_err(|error| error.to_string())?;
    let paths = DesktopPaths::from_directory(&bin_dir).map_err(|error| error.to_string())?;
    let mut controller = DesktopController::new(
        paths,
        &PathBuf::from(&state_dir),
        &PathBuf::from(&reservation_base),
        &PathBuf::from(&repository),
    )
    .map_err(|error| error.to_string())?;
    controller
        .begin_generation()
        .map_err(|error| error.to_string())?;
    *lock_session(&state) = Some(controller);
    Ok("started".into())
}

/// Runs the first half of the demonstration: open the repository,
/// describe the task, START the dispatch. Journaled — survives a crash.
#[tauri::command]
fn start_demo(state: tauri::State<Session>) -> Result<String, String> {
    let mut guard = lock_session(&state);
    let controller = guard.as_mut().ok_or("no session")?;
    controller.start_demo().map_err(|error| error.to_string())
}

/// Follows the durable evidence trail to the head; returns the reached
/// journal cursor.
#[tauri::command]
fn read_journal(state: tauri::State<Session>) -> Result<u64, String> {
    let mut guard = lock_session(&state);
    let controller = guard.as_mut().ok_or("no session")?;
    controller.read_journal().map_err(|error| error.to_string())
}

/// Runs the second half: execute the started dispatch and read the
/// completion evidence and worktree diff.
#[tauri::command]
fn finish_demo(state: tauri::State<Session>, dispatch_id: String) -> Result<String, String> {
    let mut guard = lock_session(&state);
    let controller = guard.as_mut().ok_or("no session")?;
    let outcome = controller
        .finish_demo(&dispatch_id)
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&outcome).map_err(|error| error.to_string())
}

/// The driver's journal cursor for the demo project.
#[tauri::command]
fn journal_position(state: tauri::State<Session>) -> u64 {
    let guard = lock_session(&state);
    guard.as_ref().map(|c| c.journal_position()).unwrap_or(0)
}

/// Graceful shutdown: the daemon exits cleanly; the journaled state is
/// identical to what a crash would have preserved.
#[tauri::command]
fn stop_session(state: tauri::State<Session>) -> Result<String, String> {
    let mut guard = lock_session(&state);
    if let Some(mut controller) = guard.take() {
        controller.stop();
    }
    Ok("stopped".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Session(Mutex::new(None)))
        .manage(PreviewDocument(Mutex::new(String::new())))
        // The Preview protocol serves the inert document built by
        // [`preview_document`] and nothing else: no app assets, no
        // filesystem paths, one header-bound CSP.
        // The scheme is app-global on Linux (shared web context): any
        // window may NAVIGATE to it, but the document is inert and the
        // Preview window is the only one pointed here.
        .register_uri_scheme_protocol("preview", |ctx, _request| {
            let state = ctx.app_handle().state::<PreviewDocument>();
            let body = preview_lock(&state).clone().into_bytes();
            tauri::http::Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .header(
                    "Content-Security-Policy",
                    "default-src 'none'; form-action 'none'; frame-ancestors 'none'; base-uri 'none'",
                )
                .header("X-Content-Type-Options", "nosniff")
                .header("Cache-Control", "no-store")
                .body(body)
                .unwrap_or_else(|_| {
                    tauri::http::Response::builder()
                        .status(500)
                        .header("Content-Type", "text/plain; charset=utf-8")
                        .body(b"preview unavailable".to_vec())
                        .expect("the static error response always builds")
                })
        })
        .invoke_handler(tauri::generate_handler![
            begin_session,
            start_demo,
            read_journal,
            finish_demo,
            journal_position,
            stop_session,
            open_preview
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Symbiote desktop shell");
}

#[cfg(test)]
mod preview_tests {
    use super::preview_document;

    /// Worker output is UNTRUSTED: a report containing markup must
    /// render as text, never execute. Everything is escaped and the
    /// document carries a deny-all CSP with no script.
    #[test]
    fn worker_content_renders_as_inert_text() {
        let document = preview_document(
            "disp_staffing-task",
            "<script>alert(1)</script> & <b>bold</b>",
            "/tmp/worktree",
            &[
                "produced.txt".to_string(),
                "<img src=x onerror=alert(2)>".to_string(),
            ],
        );
        assert!(!document.contains("<script>"), "{document}");
        assert!(!document.contains("<img"), "{document}");
        assert!(document.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(document.contains("<li>&lt;img src=x onerror=alert(2)&gt;</li>"));
        assert!(document.contains("default-src 'none'"));
        assert!(document.contains("Run <code>disp_staffing-task</code>"));
        assert!(!document.to_lowercase().contains("<script"));
    }

    /// The document's own structure must survive any input: the CSP and
    /// the isolation statement are present for the empty case too.
    #[test]
    fn an_empty_outcome_still_carries_the_isolation_statement() {
        let document = preview_document("", "", "", &[]);
        assert!(document.contains("default-src 'none'"));
        assert!(document.contains("no application commands"));
        assert!(document.contains("Files produced (0)"));
    }
}

/// The ACL pin (#54 review P1): the app defines an ACL manifest for its
/// commands (build.rs `app_manifest`), so Tauri's dispatch-level
/// capability gate APPLIES to app commands. The generated artifacts pin
/// both halves of the property: the manifest exists (the gate is
/// active), and the resolved capabilities grant the commands to the
/// owner window only — the Preview window is named in none.
#[cfg(test)]
mod preview_ipc_tests {
    #[test]
    fn the_app_acl_manifest_exists_and_grants_only_the_owner_window() {
        let manifests: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(env!("SYMBIOTE_ACL_MANIFESTS"))
                .expect("the generated ACL manifests must exist"),
        )
        .expect("the ACL manifests must parse");
        let app_acl = manifests
            .get("__app-acl__")
            .expect("the app ACL manifest must exist: without it Tauri's command gate never applies to app commands and any window could invoke them");
        for command in [
            "open_preview",
            "begin_session",
            "start_demo",
            "read_journal",
            "finish_demo",
            "journal_position",
            "stop_session",
        ] {
            // tauri-build normalizes generated permission identifiers
            // to hyphenated form (capability identifiers cannot carry
            // underscores).
            let allow = format!("allow-{}", command.replace('_', "-"));
            assert!(
                app_acl["permissions"].get(&allow).is_some(),
                "the generated permission {allow} must exist"
            );
        }
        let capabilities: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(env!("SYMBIOTE_RESOLVED_CAPABILITIES"))
                .expect("the resolved capabilities must exist"),
        )
        .expect("the resolved capabilities must parse");
        let capabilities = capabilities
            .as_object()
            .expect("resolved capabilities object");
        for (identifier, capability) in capabilities {
            let windows = capability
                .get("windows")
                .and_then(|windows| windows.as_array())
                .expect("every resolved capability names its windows");
            assert!(
                windows.iter().any(|window| window.as_str() == Some("main")),
                "the owner window must be granted its commands ({identifier})"
            );
            assert!(
                !windows
                    .iter()
                    .any(|window| window.as_str() == Some("preview")),
                "capability {identifier} must not grant the preview window"
            );
        }
    }
}
