//! The published canonical schema (#181). With no arguments it prints the request, response and
//! telemetry schemas derived from this crate's wire types. `--write` regenerates the committed
//! artifact and `--check` reports drift without writing, matching how this repository guards its
//! other generated documents; drift under `--check` exits non-zero, so this example gates a build
//! as well as producing the artifact. The bytes come from `symbiote_protocol::schema_json`, which
//! the case beside this crate compares the committed artifact against.

use std::path::PathBuf;
use symbiote_protocol::{SCHEMA_PATH, schema_drift, schema_json, workspace_root};

fn main() {
    let json = schema_json();
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let target = |index: usize| -> PathBuf {
        arguments
            .get(index)
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace_root().join(SCHEMA_PATH))
    };
    match arguments.first().map(String::as_str) {
        Some("--write") => {
            let path = target(1);
            std::fs::write(&path, &json).expect("write the canonical schema");
            println!("wrote {}", path.display());
        }
        Some("--check") => {
            let path = target(1);
            match schema_drift(&path) {
                Ok(()) => println!("{} matches the tree", path.display()),
                Err(reason) => {
                    eprintln!("{reason}");
                    std::process::exit(1);
                }
            }
        }
        _ => print!("{json}"),
    }
}
