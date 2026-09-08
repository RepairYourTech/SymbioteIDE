use std::io::{Read, Write};

fn main() {
    if let Err(error) = run() {
        eprintln!("symbiote: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 3 || arguments[0] != "--state-dir" || arguments[2] != "request" {
        return Err("usage: symbiote --state-dir PRIVATE_DIRECTORY request < request.json".into());
    }
    let mut input = Vec::new();
    std::io::stdin().take(65_537).read_to_end(&mut input)?;
    if input.len() > 65_536 {
        return Err("request exceeds 64 KiB".into());
    }
    // Pretty printed request files are permitted; transport framing remains one line.
    let value = symbiote_protocol::parse_request(&input)?;
    let request = serde_json::to_vec(&value)?;
    let response =
        symbiote_host::transport::exchange(std::path::Path::new(&arguments[1]), &request)?;
    std::io::stdout().write_all(&response)?;
    let response: symbiote_protocol::Response = serde_json::from_slice(&response)?;
    response.result?;
    Ok(())
}
