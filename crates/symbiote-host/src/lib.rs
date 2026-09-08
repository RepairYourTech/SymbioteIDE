//! Headless local control-plane service. Runtime execution is not enabled here.
#[cfg(not(target_os = "linux"))]
compile_error!(
    "symbiote-host currently requires Linux SO_PEERCRED; other transports remain unimplemented"
);

mod service;
pub mod transport;

use std::path::Path;
use symbiote_domain::UserId;
use symbiote_protocol::{
    CURRENT_VERSION, ErrorCode, Principal, ProtocolError, Response, encode_response, parse_request,
};
use symbiote_store::Store;

pub fn serve(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let local = transport::LocalListener::bind(directory)?;
    // The same authenticated OS owner is the explicit bootstrap policy. This
    // adapter must never be exposed as an unauthenticated remote or Preview API.
    let principal = Principal::local_owner(UserId::new(format!(
        "local-uid-{}",
        nix::unistd::geteuid()
    ))?);
    let mut store = Store::open(directory.join("control.sqlite3"))?;
    eprintln!("symbioted: ready (local metadata capabilities only)");
    for connection in local.listener.incoming() {
        let connection = connection?;
        if transport::LocalListener::authenticate(&connection).is_err() {
            continue;
        }
        let request = transport::read_frame(&connection)
            .map_err(|_| ProtocolError::new(ErrorCode::InvalidRequest))
            .and_then(|bytes| parse_request(&bytes));
        let (response, shutdown) = match request {
            Ok(request) => service::handle(&mut store, &principal, request),
            Err(error) => (
                Response {
                    version: CURRENT_VERSION,
                    correlation_id: None,
                    result: Err(error),
                },
                false,
            ),
        };
        let mut bytes = match encode_response(&response) {
            Ok(bytes) => bytes,
            Err(error) => {
                encode_response(&Response::failure(response.correlation_id.clone(), error))?
            }
        };
        bytes.push(b'\n');
        // A disconnected client does not roll back an already committed command.
        // Retrying its command ID recovers the durable receipt.
        if transport::write_response(&connection, &bytes).is_err() {
            eprintln!("symbioted: response disconnected; command disposition retained");
        }
        if shutdown {
            break;
        }
    }
    Ok(())
}
