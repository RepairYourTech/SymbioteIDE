//! One request/response exchange with the daemon: the request's identity, the
//! socket round trip, and the rendering of whatever came back.
//!
//! Everything here runs only after the gate has admitted the operation, so a
//! refusal never reaches this module and never opens the socket. `options.json`
//! selects between the versioned envelope (one per invocation, for automation)
//! and the daemon's own pretty-printed body (for a human).
use std::sync::atomic::{AtomicU64, Ordering};

use crate::args::{EXIT_OK, EXIT_REFUSED, EXIT_USAGE, Options};
use crate::output::{error_envelope, success_envelope};

/// Distinguishes repeated minting inside one process. Its absence is a real
/// hazard rather than a theoretical one: two invocations in the same
/// millisecond would otherwise share an idempotency key, and the daemon would
/// treat the second, different intent as a replay of the first.
static NEXT_INVOCATION: AtomicU64 = AtomicU64::new(0);

/// A caller-supplied `--command-id` restores the documented lost-response
/// retry contract: the same id with identical intent replays the durable
/// receipt instead of re-executing. The default mints an id that is unique
/// per invocation — millisecond, pid, and an in-process sequence — so
/// parallel scripts collide neither across processes nor within one.
pub(crate) fn minted_command_id(options: &Options, name: &str) -> String {
    match &options.command_id_override {
        Some(id) => id.clone(),
        None => format!(
            "cli-{name}-{}-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default(),
            std::process::id(),
            NEXT_INVOCATION.fetch_add(1, Ordering::Relaxed)
        ),
    }
}

/// Sends one admitted operation and renders the result. The state directory is
/// required only here, which is why `symbiote help` and `symbiote schema` — and
/// any refusal — work without one.
pub(crate) fn send(
    name: &str,
    options: &Options,
    operation: serde_json::Map<String, serde_json::Value>,
) -> Result<i32, Box<dyn std::error::Error>> {
    let Some(directory) = options.state_dir.clone() else {
        eprintln!("symbiote: missing --state-dir PRIVATE_DIRECTORY");
        return Ok(EXIT_USAGE);
    };
    let command_id = minted_command_id(options, name);
    let request = serde_json::json!({
        "version": symbiote_protocol::CURRENT_VERSION,
        "correlation_id": command_id,
        "command_id": command_id,
        "operation": operation,
    });
    let bytes = serde_json::to_vec(&request)?;
    let response_bytes = match symbiote_host::transport::exchange(&directory, &bytes) {
        Ok(bytes) => bytes,
        Err(error) => {
            let message = format!(
                "cannot reach the daemon at {}: {error} (is symbioted running with --state-dir {}?)",
                directory.join("host.sock").display(),
                directory.display()
            );
            if options.json {
                println!(
                    "{}",
                    error_envelope(name, &command_id, "unreachable", &message)
                );
                return Ok(EXIT_USAGE);
            }
            eprintln!("symbiote: {message}");
            return Ok(EXIT_USAGE);
        }
    };
    let response: symbiote_protocol::Response = match serde_json::from_slice(&response_bytes) {
        Ok(response) => response,
        Err(error) => {
            let message = format!("daemon sent an unparseable response: {error}");
            if options.json {
                println!(
                    "{}",
                    error_envelope(name, &command_id, "unreachable", &message)
                );
                return Ok(EXIT_USAGE);
            }
            eprintln!("symbiote: {message}");
            return Ok(EXIT_USAGE);
        }
    };
    match &response.result {
        Ok(body) => {
            let value = serde_json::to_value(body).unwrap_or(serde_json::Value::Null);
            if options.json {
                println!("{}", success_envelope(name, &command_id, &value));
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(body).unwrap_or_else(|_| format!("{body:?}"))
                );
            }
            Ok(EXIT_OK)
        }
        Err(error) => {
            if options.json {
                let code = serde_json::to_value(error.code)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "internal".into());
                println!(
                    "{}",
                    error_envelope(name, &command_id, &code, &error.message)
                );
            } else {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(error).unwrap_or_else(|_| format!("{error:?}"))
                );
            }
            Ok(EXIT_REFUSED)
        }
    }
}
