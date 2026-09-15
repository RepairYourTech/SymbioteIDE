//! The published domain ontology (#36).
//!
//! With no arguments it prints the ontology in the committed encoding. `--write`
//! regenerates the committed artifact and `--check` reports drift without
//! writing, matching how this repository guards its other generated documents.
//! Any problem in the vocabulary — or drift under `--check` — exits non-zero, so
//! this example gates a build as well as producing the artifact.

use std::path::PathBuf;
use symbiote_domain::{
    IDENTITY_NAMES, ONTOLOGY_SCHEMA_PATH, OUTSTANDING, VOCABULARY, ontology_schema, ontology_text,
    problems, workspace_root,
};

fn main() {
    let root = workspace_root();
    let json = ontology_text();
    let violations = problems(VOCABULARY, OUTSTANDING, IDENTITY_NAMES, &ontology_schema());
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let target = |index: usize| -> PathBuf {
        arguments
            .get(index)
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join(ONTOLOGY_SCHEMA_PATH))
    };
    let mut drifted = false;
    match arguments.first().map(String::as_str) {
        Some("--write") => {
            let path = target(1);
            std::fs::write(&path, &json).expect("write the ontology");
            println!("wrote {}", path.display());
        }
        Some("--check") => {
            let path = target(1);
            let committed = std::fs::read_to_string(&path).unwrap_or_default();
            if committed == json {
                println!("{} matches the tree", path.display());
            } else {
                eprintln!("{} differs from the tree", path.display());
                drifted = true;
            }
        }
        _ => print!("{json}"),
    }
    for violation in &violations {
        eprintln!("{violation}");
    }
    if !violations.is_empty() {
        eprintln!("{} vocabulary problem(s)", violations.len());
        std::process::exit(1);
    }
    if drifted {
        std::process::exit(1);
    }
}
