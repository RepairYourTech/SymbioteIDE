fn main() {
    if let Err(error) = run() {
        eprintln!("symbioted: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if !(arguments.len() == 2 || (arguments.len() == 3 && arguments[2] == "--no-telemetry"))
        || arguments[0] != "--state-dir"
    {
        return Err("usage: symbioted --state-dir PRIVATE_DIRECTORY [--no-telemetry]".into());
    }
    symbiote_host::serve_with_telemetry(std::path::Path::new(&arguments[1]), arguments.len() == 2)
}
