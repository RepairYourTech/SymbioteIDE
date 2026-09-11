//! Presentation: the versioned envelopes the CLI prints, and the interactive
//! confirmation prompt.
//!
//! Every `--json` invocation prints exactly one of these envelopes on stdout,
//! so their shape is the CLI's published contract — the fields here are the
//! ones `docs/contracts/schemas/symbiote.cli.v1.schema.json` declares, and the
//! fixture test binds the two.
use std::io::Write;

use symbiote_host::cli_authorization::confirmation_accepted;
use symbiote_host::cli_schema::CLI_SCHEMA;

/// The versioned success envelope: the daemon's own body, unmodified.
pub(crate) fn success_envelope(
    command: &str,
    command_id: &str,
    result: &serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "schema": CLI_SCHEMA,
        "command": command,
        "command_id": command_id,
        "ok": true,
        "result": result,
    })
}

/// The versioned failure envelope. `code` is the daemon's own error code, or
/// one of the CLI's own: `authorization_required` (nothing was sent) and
/// `unreachable` (the daemon could not be reached).
pub(crate) fn error_envelope(
    command: &str,
    command_id: &str,
    code: &str,
    message: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema": CLI_SCHEMA,
        "command": command,
        "command_id": command_id,
        "ok": false,
        "error": { "code": code, "message": message },
    })
}

/// Names BOTH the typed command and the operation that makes it dangerous:
/// for `raw`, the operation is the whole story and the operator must see it.
pub(crate) fn confirm(command: &str, kind: &str) -> bool {
    eprint!("symbiote: {command} sends the dangerous operation {kind}; type \"yes\" to continue: ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => false,
        Ok(_) => confirmation_accepted(&line),
    }
}
