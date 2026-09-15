//! The per-invariant conformance report (#170).
//!
//! With no arguments it prints the report in the committed encoding. `--write`
//! regenerates the committed artifact, and `--check` reports drift without
//! writing, matching how this repository guards its other generated documents.
//! Any failed channel — or drift under `--check` — exits non-zero, so the
//! example gates a build as well as producing evidence.

use std::path::PathBuf;
use symbiote_constitution::{REPORT_PATH, evaluate, report_to_json, workspace_root};

fn main() {
    let root = workspace_root();
    let report = evaluate(&root);
    let json = report_to_json(&report);
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let path = |index: usize| -> PathBuf {
        arguments
            .get(index)
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(REPORT_PATH))
    };
    let mut drifted = false;
    match arguments.first().map(String::as_str) {
        Some("--write") => {
            let target = path(1);
            std::fs::write(&target, &json).expect("write the report");
            println!("wrote {}", target.display());
        }
        Some("--check") => {
            let target = path(1);
            let committed = std::fs::read_to_string(&target).unwrap_or_default();
            if committed == json {
                println!("{} matches the tree", target.display());
            } else {
                eprintln!("{} differs from the tree", target.display());
                drifted = true;
            }
        }
        _ => print!("{json}"),
    }
    if !report.failures.is_empty() {
        eprintln!("{} channel(s) failed", report.failures.len());
        std::process::exit(1);
    }
    if drifted {
        std::process::exit(1);
    }
}
