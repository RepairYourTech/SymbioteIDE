fn main() {
    if let Err(error) = run() {
        eprintln!("symbioted: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 || arguments[0] != "--state-dir" {
        return Err("usage: symbioted --state-dir PRIVATE_DIRECTORY".into());
    }
    symbiote_host::serve(std::path::Path::new(&arguments[1]))
}
