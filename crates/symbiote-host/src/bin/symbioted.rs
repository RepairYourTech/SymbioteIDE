// The record of the sources this build compiled, written by the build script
// into OUT_DIR and included here so it travels inside the binary. A proof can
// then tell whether the daemon it is about to drive was built from the sources
// under test, without a record file that a later build could refresh on its
// own. It is a `static` in the binary's root because a `static` in the library
// is dropped when this crate links it without referencing it. See
// `symbiote-source-stamp`, which names this file.
include!(concat!(env!("OUT_DIR"), "/source_record.rs"));

fn main() {
    // Read, not merely declared: a static the binary never reads is one the
    // compiler or linker may drop, and Rust 1.85 does drop this one (`#[used]`
    // did not keep it, and the desktop proofs refused its daemon for carrying
    // no record at all). A record that can be dropped cannot bind a binary to
    // the sources a proof drives it against.
    std::hint::black_box(SOURCE_RECORD);

    if let Err(error) = run() {
        eprintln!("symbioted: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let arguments: Vec<String> = arguments
        .into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| "arguments must be UTF-8".to_string())
        })
        .collect::<Result<_, _>>()?;
    let telemetry = !arguments.iter().any(|a| a == "--no-telemetry");
    let positional: Vec<&String> = arguments
        .iter()
        .filter(|a| a.as_str() != "--no-telemetry")
        .collect();
    // --state-dir DIR [--operator-config FILE]: the operator config is the
    // explicit provisioning surface (reservation base, fixture model
    // transport, credential registrations, shell allowlist). Its values
    // never enter the journal.
    match positional.as_slice() {
        [state_flag, state_dir] if state_flag.as_str() == "--state-dir" => {
            symbiote_host::serve_with_telemetry(std::path::Path::new(state_dir), telemetry)
        }
        [state_flag, state_dir, config_flag, config_path]
            if state_flag.as_str() == "--state-dir"
                && config_flag.as_str() == "--operator-config" =>
        {
            let config = symbiote_host::operator::load(std::path::Path::new(config_path))
                .map_err(|error| format!("operator config: {error}"))?;
            let transports = symbiote_host::runner::assemble_operator_transports(config)?;
            symbiote_host::serve_full(std::path::Path::new(state_dir), telemetry, transports)
        }
        _ => Err("usage: symbioted --state-dir PRIVATE_DIRECTORY [--operator-config FILE] [--no-telemetry]".into()),
    }
}
