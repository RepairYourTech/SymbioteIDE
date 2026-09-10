//! The Symbiote desktop shell: Tauri commands over the headless
//! [`controller`]. The commands are deliberately thin — every behavior
//! they expose is the controller's, which is proven against a real
//! daemon in headless tests. The UI never sees secrets, daemon state
//! internals, or anything but the sequencer's results.
pub mod controller;

use controller::{DesktopController, DesktopError, DesktopPaths};
use std::{path::PathBuf, sync::Mutex};

/// The managed controller. `None` until the operator begins a session.
struct Session(Mutex<Option<DesktopController>>);

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
    *state.0.lock().expect("session lock") = Some(controller);
    Ok("started".into())
}

/// Runs the first half of the demonstration: open the repository,
/// describe the task, START the dispatch. Journaled — survives a crash.
#[tauri::command]
fn start_demo(state: tauri::State<Session>) -> Result<String, String> {
    let mut guard = state.0.lock().expect("session lock");
    let controller = guard.as_mut().ok_or("no session")?;
    controller.start_demo().map_err(|error| error.to_string())
}

/// Follows the durable evidence trail to the head; returns the reached
/// journal cursor.
#[tauri::command]
fn read_journal(state: tauri::State<Session>) -> Result<u64, String> {
    let mut guard = state.0.lock().expect("session lock");
    let controller = guard.as_mut().ok_or("no session")?;
    controller.read_journal().map_err(|error| error.to_string())
}

/// Runs the second half: execute the started dispatch and read the
/// completion evidence and worktree diff.
#[tauri::command]
fn finish_demo(state: tauri::State<Session>, dispatch_id: String) -> Result<String, String> {
    let mut guard = state.0.lock().expect("session lock");
    let controller = guard.as_mut().ok_or("no session")?;
    let outcome = controller
        .finish_demo(&dispatch_id)
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&outcome).map_err(|error| error.to_string())
}

/// The driver's journal cursor for the demo project.
#[tauri::command]
fn journal_position(state: tauri::State<Session>) -> u64 {
    let guard = state.0.lock().expect("session lock");
    guard.as_ref().map(|c| c.journal_position()).unwrap_or(0)
}

/// Graceful shutdown: the daemon exits cleanly; the journaled state is
/// identical to what a crash would have preserved.
#[tauri::command]
fn stop_session(state: tauri::State<Session>) -> Result<String, String> {
    let mut guard = state.0.lock().expect("session lock");
    if let Some(mut controller) = guard.take() {
        controller.stop();
    }
    Ok("stopped".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Session(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            begin_session,
            start_demo,
            read_journal,
            finish_demo,
            journal_position,
            stop_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Symbiote desktop shell");
}
