//! Emit the per-invariant conformance report as JSON, for publication as
//! evidence against #170. Exit status is non-zero when any channel failed, so
//! the example can gate a build as well as produce evidence.

fn main() {
    let report = symbiote_constitution::evaluate(&symbiote_constitution::workspace_root());
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serialization")
    );
    if !report.failures.is_empty() {
        eprintln!("{} channel(s) failed", report.failures.len());
        std::process::exit(1);
    }
}
