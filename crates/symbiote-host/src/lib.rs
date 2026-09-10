//! Headless local control-plane service. Runtime execution is not enabled here.
#[cfg(not(target_os = "linux"))]
compile_error!(
    "symbiote-host currently requires Linux SO_PEERCRED; other transports remain unimplemented"
);

/// Context/credential resolution wiring (#217): the store-backed
/// `ContextSource` and the dispatch-facet resolution the activation path
/// uses. Credential VALUES live only in the operator's broker.
pub(crate) mod context_resolution;
mod identity;
mod inventory;
pub mod runner;
mod service;
/// The sandboxed shell-tool executor composition (#218/#465): production
/// `ShellToolExecutor` over `symbiote-sandbox::launch` plus the operator's
/// consent authority seam.
pub mod shell_executor;
pub mod transport;

use std::path::Path;
use symbiote_domain::UserId;
use symbiote_protocol::{
    CURRENT_VERSION, ErrorCode, Principal, ProtocolError, Response, encode_response, parse_request,
};
use symbiote_store::Store;

pub fn serve(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    serve_with_telemetry(directory, true)
}

pub fn serve_with_telemetry(
    directory: &Path,
    telemetry: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let local = transport::LocalListener::bind(directory)?;
    let mut inventory = inventory::InventoryService::new(identity::load(directory)?, telemetry)?;
    // The same authenticated OS owner is the explicit bootstrap policy. This
    // adapter must never be exposed as an unauthenticated remote or Preview API.
    let principal = Principal::local_owner(UserId::new(format!(
        "local-uid-{}",
        nix::unistd::geteuid()
    ))?);
    let mut store = Store::open(directory.join("control.sqlite3"))?;
    // Production configuration: no live worker transports are configured.
    // Activation of a started dispatch refuses with a typed error until the
    // operator provisions an authorized transport path (sandboxed launch +
    // credential/billing authorization).
    let mut worker_transports = runner::WorkerTransports::production();
    eprintln!("symbioted: ready (local metadata capabilities only)");
    // Known limitation: connections are served synchronously. With no
    // worker transports configured this is irrelevant (activation refuses
    // before any loop runs); if an operator ever provisions a transport,
    // turns must move off this loop or every connection stalls for the
    // turn's duration. See the worker-activation contract notes.
    for connection in local.listener.incoming() {
        let connection = connection?;
        if transport::LocalListener::authenticate(&connection).is_err() {
            continue;
        }
        let request = transport::read_frame(&connection)
            .map_err(|_| ProtocolError::new(ErrorCode::InvalidRequest))
            .and_then(|bytes| parse_request(&bytes));
        let (response, shutdown) = match request {
            Ok(request) => service::handle(
                &mut store,
                &principal,
                &mut inventory,
                &mut worker_transports,
                request,
            ),
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
