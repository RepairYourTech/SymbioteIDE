//! The Symbiote desktop shell entry point. The binary is a thin Tauri
//! shell over [`symbiote_desktop_lib`]; all behavior lives in the
//! headless controller so it can be proven without a display.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    symbiote_desktop_lib::run()
}
