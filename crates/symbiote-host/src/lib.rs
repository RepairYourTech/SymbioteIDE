//! Headless local control-plane service. Runtime execution is not enabled here.
#[cfg(not(target_os = "linux"))]
compile_error!(
    "symbiote-host currently requires Linux SO_PEERCRED; other transports remain unimplemented"
);

/// The `symbiote` CLI's published contract (#54): the schema identities, the
/// operation-risk classification those identities describe, and the JSON
/// Schema documents the binary emits. One owner for what the CLI promises
/// automation, shared by the binary and its test suites.
pub mod cli_schema;

/// The `symbiote` CLI's authorization engine (#54): the policy document the
/// CLI's `--policy` flag honors, and the pure decision that turns a risk
/// classification plus the grants an operator configured into an answer. Kept
/// beside the schema that publishes the very document it enforces.
pub mod cli_authorization;

/// Context/credential resolution wiring (#217): the store-backed
/// `ContextSource` and the dispatch-facet resolution the activation path
/// uses. Credential VALUES live only in the operator's broker.
pub(crate) mod context_resolution;
mod identity;
mod inventory;
/// Declared resource limits and the bound the Host applies for them: the
/// dispatch's intent beside the Host's answer, consulted before anything runs.
pub mod limits;
/// Operator provisioning (#54): the explicit configuration file that turns
/// dispatch activation into a working execution path.
pub mod operator;
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
    serve_full(directory, telemetry, runner::WorkerTransports::production())
}

/// Serves with operator-provisioned worker transports (#54): the
/// configuration is trusted operator state assembled by the binary — this
/// entry point never reads client input.
pub fn serve_full(
    directory: &Path,
    telemetry: bool,
    mut worker_transports: runner::WorkerTransports,
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
    if worker_transports.reservation_base().is_ok() {
        eprintln!(
            "symbioted: operator provisioning active (reservation base + configured transports)"
        );
    } else {
        eprintln!("symbioted: ready (local metadata capabilities only)");
    }
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
