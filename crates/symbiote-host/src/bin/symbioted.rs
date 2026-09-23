// The record of the sources this build compiled, written by the build script
// into OUT_DIR and included here so it travels inside the binary. A proof can
// then tell whether the daemon it is about to drive was built from the sources
// under test, without a record file that a later build could refresh on its
// own. It is a `static` in the binary's root because a `static` in the library
// is dropped when this crate links it without referencing it. See
// `symbiote-source-stamp`, which names this file.
include!(concat!(env!("OUT_DIR"), "/source_record.rs"));

use std::path::PathBuf;

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

/// The path a flag's value names, refusing a flag that ends the invocation.
fn value_of(arguments: &[String], index: &mut usize) -> Result<PathBuf, String> {
    *index += 1;
    arguments
        .get(*index)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{} needs a path", arguments[*index - 1]))
}

const USAGE: &str = "usage: symbioted [--state-dir DIR] [--config-dir DIR] [--operator-config FILE] \
                     [--no-telemetry]\n\
                     without --state-dir the Host uses $SYMBIOTE_HOST_STATE_DIR, else \
                     $XDG_STATE_HOME/symbiote, else $HOME/.local/state/symbiote; without \
                     --config-dir or --operator-config it looks for operator.json under \
                     $SYMBIOTE_HOST_CONFIG_DIR, else $XDG_CONFIG_HOME/symbiote, else \
                     $HOME/.config/symbiote (a missing file is no operator provisioning)";

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| "arguments must be UTF-8".to_string())
        })
        .collect::<Result<_, _>>()?;

    let mut telemetry = true;
    let mut state_dir: Option<PathBuf> = None;
    let mut config_dir: Option<PathBuf> = None;
    let mut operator_config: Option<PathBuf> = None;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--no-telemetry" => telemetry = false,
            "--state-dir" => state_dir = Some(value_of(&arguments, &mut index)?),
            "--config-dir" => config_dir = Some(value_of(&arguments, &mut index)?),
            "--operator-config" => operator_config = Some(value_of(&arguments, &mut index)?),
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other:?}\n{USAGE}").into()),
        }
        index += 1;
    }

    // One resolution, from the flag or this Host's override or the XDG base
    // directories: the daemon binds its socket, opens its database and looks
    // for the operator configuration under the directories this names.
    let paths =
        symbiote_host::paths::HostPaths::from_process(state_dir.as_deref(), config_dir.as_deref())?;
    let config = operator_config.unwrap_or_else(|| paths.operator_config());
    let transports = match config.exists() {
        true => {
            let config = symbiote_host::operator::load(&config)
                .map_err(|error| format!("operator config {}: {error}", config.display()))?;
            symbiote_host::runner::assemble_operator_transports(config)?
        }
        false => symbiote_host::runner::WorkerTransports::production(),
    };
    symbiote_host::serve_full(&paths.state, telemetry, transports)
}
