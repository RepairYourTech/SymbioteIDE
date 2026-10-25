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
/// The production external-harness transport (#218/#465): the operator's
/// configured harness program launched inside the sandbox under the
/// dispatch's declared memory ceiling.
pub mod external_harness;
mod identity;
mod inventory;
/// Declared resource limits and the bound the Host applies for them: the
/// dispatch's intent beside the Host's answer, consulted before anything runs.
pub mod limits;
/// Operator provisioning (#54): the explicit configuration file that turns
/// dispatch activation into a working execution path.
pub mod operator;
/// The Host's own directories (#180): the XDG base directories, the explicit overrides, and the
/// refusals that keep an unresolved Host from writing where nobody intended.
pub mod paths;
pub mod runner;
/// The runtime inventory this Host publishes and serves (#186): the private,
/// bounded file a discovery document is installed into, and the re-validated
/// read the `get_runtime_inventory` operation answers from.
pub mod runtime_inventory;
mod service;
/// The sandboxed shell-tool executor composition (#218/#465): production
/// `ShellToolExecutor` over `symbiote-sandbox::launch` plus the operator's
/// consent authority seam.
pub mod shell_executor;
pub mod transport;

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use symbiote_domain::{HostId, UserId};
use symbiote_protocol::{
    CURRENT_VERSION, ErrorCode, Principal, ProtocolError, Response, encode_response, parse_request,
};
use symbiote_store::Store;

/// This Host's own stable identity for a state directory, from the private file
/// the daemon loads at startup. The CLI's local `publish-runtime-inventory`
/// command publishes into the same directory, so it has to resolve the same
/// identity the daemon will require every record to name — resolving it twice
/// would be two owners for one fact.
pub fn host_identity(directory: &Path) -> Result<HostId, Box<dyn std::error::Error>> {
    Ok(identity::load(directory)?)
}

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
    worker_transports: runner::WorkerTransports,
) -> Result<(), Box<dyn std::error::Error>> {
    let local = transport::LocalListener::bind(directory)?;
    let host_id = identity::load(directory)?;
    let inventory = inventory::InventoryService::new(
        host_id.clone(),
        telemetry,
        runtime_inventory::Published::new(directory, host_id),
    )?;
    // The same authenticated OS owner is the explicit bootstrap policy. This
    // adapter must never be exposed as an unauthenticated remote or Preview API.
    let principal = Principal::local_owner(UserId::new(format!(
        "local-uid-{}",
        nix::unistd::geteuid()
    ))?);
    let store = Store::open(directory.join(paths::DATABASE_FILE))?;
    if worker_transports.reservation_base().is_ok() {
        eprintln!(
            "symbioted: operator provisioning active (reservation base + configured transports)"
        );
    } else {
        eprintln!("symbioted: ready (local metadata capabilities only)");
    }
    let serving = Arc::new(Serving {
        store: Mutex::new(store),
        inventory: Mutex::new(inventory),
        workers: Mutex::new(worker_transports),
        running: service::empty_running_dispatches(),
    });
    let shutdown = Arc::new(AtomicBool::new(false));
    let in_flight = Arc::new(AtomicUsize::new(0));
    // The accept loop polls rather than blocks, because a shutdown request is
    // served on a CONNECTION thread and has to be able to stop the loop that
    // is waiting for the next connection. Blocking in `accept` would leave the
    // daemon running until some unrelated client happened to connect.
    local.listener.set_nonblocking(true)?;
    // The connection threads this loop spawned, so a shutdown can wait for
    // the turns those connections are still running: a `Shutdown` served on
    // one connection stops new work here, and the joins below are the whole
    // grace period — in-flight turns finish, file their evidence, and answer
    // their callers before the process exits. Nothing is killed that the
    // operator did not kill with a signal.
    let mut threads: Vec<std::thread::JoinHandle<()>> = Vec::new();
    loop {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        // Reap the connections that have finished: an exited joinable thread
        // keeps its stack mapped until it is joined or detached, and a Host
        // that is polled connection-per-request would otherwise hold one dead
        // stack per poll for its whole life. Finished handles drop (detaching
        // threads that have already exited), live ones stay for the shutdown
        // joins below.
        threads.retain(|handle| !handle.is_finished());
        match local.listener.accept() {
            Ok((connection, _)) => {
                if in_flight.load(Ordering::SeqCst) >= MAX_IN_FLIGHT_CONNECTIONS {
                    // At capacity the connection is REFUSED, by name, on the
                    // spot: serving it here on the accept loop would let one
                    // slow client freeze every client that connects after it,
                    // and a silent close would be an unexplained failure for a
                    // client that did nothing wrong. `resource_exhausted` is
                    // the protocol's own word for a bound reached, and the
                    // client's retry is the remedy.
                    refuse_exhausted(&connection);
                    continue;
                }
                in_flight.fetch_add(1, Ordering::SeqCst);
                let serving_thread = Arc::clone(&serving);
                let principal_thread = principal.clone();
                let shutdown_thread = Arc::clone(&shutdown);
                let in_flight_thread = Arc::clone(&in_flight);
                threads.push(std::thread::spawn(move || {
                    // The decrement rides a guard rather than a trailing
                    // statement: a connection that panics unwinds through
                    // this drop, so a slot is never leaked by a thread that
                    // died — and a leaked slot is a capacity the Host never
                    // gives back.
                    let _slot = InFlightGuard(&in_flight_thread);
                    serve_connection(
                        &connection,
                        &serving_thread,
                        &principal_thread,
                        &shutdown_thread,
                    );
                }));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_POLL);
            }
            Err(error) => return Err(Box::new(error)),
        }
    }
    for thread in threads {
        // A connection thread cannot be joining itself: the loop is the only
        // joiner, and it has stopped accepting.
        let _ = thread.join();
    }
    Ok(())
}

/// The accept loop's release of a connection slot. `Drop`, because a trailing
/// decrement is a line a panic skips and a slot the Host never re-lets.
struct InFlightGuard<'a>(&'a Arc<AtomicUsize>);

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Answers one overflow connection with the protocol's own named bound and
/// closes it. Nothing about the request is read — the bound is on the Host's
/// own serving capacity, which the connection itself proves by arriving.
fn refuse_exhausted(connection: &std::os::unix::net::UnixStream) {
    let response = Response {
        version: CURRENT_VERSION,
        correlation_id: None,
        result: Err(ProtocolError {
            code: ErrorCode::ResourceExhausted,
            message: format!(
                "the Host is serving at most {MAX_IN_FLIGHT_CONNECTIONS} connections at once; retry"
            ),
        }),
    };
    let Ok(mut bytes) = encode_response(&response) else {
        return;
    };
    bytes.push(b'\n');
    let _ = transport::write_response(connection, &bytes);
}

/// How many connections the Host serves at once. Four concurrent agent
/// streams plus a client watching each of them is the workload #38 names, and
/// this sits above it; the number is a bound, not a target.
pub const MAX_IN_FLIGHT_CONNECTIONS: usize = 16;

/// How long the accept loop waits before looking again. It exists only so a
/// shutdown served on a connection thread can stop the loop promptly, so it is
/// short enough that `shutdown` is not noticeably delayed and long enough not
/// to busy-wait.
const ACCEPT_POLL: std::time::Duration = std::time::Duration::from_millis(25);

/// The Host's mutable serving state, shared by every connection thread.
///
/// The three are separate locks rather than one because they are not held for
/// the same span: `workers` builds a run's transport and is released before
/// the turn, and only `store` is taken again afterwards, to file the
/// evidence. They are locked in exactly one place — [`Serving::handle`] — and
/// always in the order store, inventory, workers, so the set cannot deadlock:
/// no other code path takes any of them, and none takes a second while
/// holding a first in a different order.
pub(crate) struct Serving {
    pub(crate) store: Mutex<Store>,
    inventory: Mutex<inventory::InventoryService>,
    pub(crate) workers: Mutex<runner::WorkerTransports>,
    running: service::RunningDispatches,
}

impl Serving {
    /// Serves one request under all three locks and releases them again. The
    /// one operation that outlives them — an agent turn — comes back as a
    /// `DeferredRun` rather than as an answer.
    pub(crate) fn handle(
        &self,
        principal: &Principal,
        request: symbiote_protocol::Request,
    ) -> (Response, bool, Option<service::DeferredRun>) {
        let mut store = self
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut inventory = self
            .inventory
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut workers = self
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        service::handle(
            &mut store,
            principal,
            &mut inventory,
            &mut workers,
            &self.running,
            request,
        )
    }
}

/// Authenticates, serves and answers one connection. Everything the request
/// needs is borrowed from `serving` and released again before the response is
/// written, so a long turn never holds a lock another client is waiting for.
fn serve_connection(
    connection: &std::os::unix::net::UnixStream,
    serving: &Serving,
    principal: &Principal,
    shutdown: &AtomicBool,
) {
    if transport::LocalListener::authenticate(connection).is_err() {
        return;
    }
    let request = transport::read_frame(connection)
        .map_err(|_| ProtocolError::new(ErrorCode::InvalidRequest))
        .and_then(|bytes| parse_request(&bytes));
    let (mut response, requested_shutdown, deferred) = match request {
        Ok(request) => serving.handle(principal, request),
        Err(error) => (
            Response {
                version: CURRENT_VERSION,
                correlation_id: None,
                result: Err(error),
            },
            false,
            None,
        ),
    };
    // Every lock `Serving::handle` took is released. The turn runs here, and
    // `resolve` re-takes only the store, for the moment it files evidence.
    if let Some(deferred) = deferred {
        response.result = deferred.resolve(&serving.store);
    }
    let mut bytes = match encode_response(&response) {
        Ok(bytes) => bytes,
        Err(error) => {
            // The response itself does not encode. The fallback failure is
            // reported on stderr rather than propagated: this connection's
            // thread has no caller to return to, and every other connection is
            // still being served.
            match encode_response(&Response::failure(response.correlation_id.clone(), error)) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("symbioted: response could not be encoded: {error}");
                    return;
                }
            }
        }
    };
    bytes.push(b'\n');
    // A disconnected client does not roll back an already committed command.
    // Retrying its command ID recovers the durable receipt.
    if transport::write_response(connection, &bytes).is_err() {
        eprintln!("symbioted: response disconnected; command disposition retained");
    }
    if requested_shutdown {
        shutdown.store(true, Ordering::SeqCst);
    }
}
