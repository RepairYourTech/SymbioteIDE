fn main() {
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
            // The same authenticated-OS-owner bootstrap policy as serve.
            let user_id = symbiote_domain::UserId::new(format!(
                "local-uid-{}",
                nix::unistd::geteuid().as_raw()
            ))
            .map_err(|_| "local principal identity")?;
            let transports =
                symbiote_host::runner::assemble_operator_transports(config, &user_id)?;
            symbiote_host::serve_full(std::path::Path::new(state_dir), telemetry, transports)
        }
        _ => Err("usage: symbioted --state-dir PRIVATE_DIRECTORY [--operator-config FILE] [--no-telemetry]".into()),
    }
}
