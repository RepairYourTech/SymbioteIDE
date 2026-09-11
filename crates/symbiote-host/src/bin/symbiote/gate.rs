//! The authorization gate: whether this invocation may send the operation it
//! built at all.
//!
//! The CLI holds no daemon authority of its own — it cannot widen what the
//! protocol permits. What it owns is the decision to *send* a dangerous
//! request, and that decision never happens silently. This module is the one
//! place that decision is made, for every route: the typed commands and `raw`
//! alike, because authorization follows the OPERATION the command emitted
//! rather than the command's name.
//!
//! The gate runs **before** anything touches the daemon, so an unauthorized
//! invocation never connects (pinned by the integration test's never-accepted
//! socket). The engine it consults — the policy document and the pure
//! decision — lives in `symbiote_host::cli_authorization`.
use std::io::IsTerminal;
use std::path::PathBuf;

use symbiote_host::cli_authorization::{
    Authorization, POLICY_ENV, PolicyError, authorize, load_policy,
};
use symbiote_host::cli_schema::risk_of_kind;

use crate::args::{EXIT_AUTHORIZATION_REQUIRED, EXIT_USAGE, Options};
use crate::output::{confirm, error_envelope};
use crate::session::minted_command_id;

/// The gate's verdict. A refusal is complete: it has already reported itself,
/// and the exit code it names is the invocation's result.
pub(crate) enum Gate {
    /// The operation is authorized; the caller may send it.
    Proceed,
    /// Refused before connecting, with the exit code the refusal decided.
    Refused(i32),
}

/// The policy path for this invocation: an explicit `--policy` wins, else
/// `SYMBIOTE_CLI_POLICY`. An empty variable means "no policy", like unset.
pub(crate) fn resolved_policy_path(options: &Options) -> Option<PathBuf> {
    options.policy.clone().or_else(|| {
        std::env::var_os(POLICY_ENV)
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
    })
}

/// Unix milliseconds, the unit a policy's `expires_at` is written in. A clock
/// before the epoch reads as 0, which only makes an expiry more likely to have
/// lapsed — the fail-closed direction.
fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or_default()
}

/// Decides whether `kind` may be sent. `--yes` outranks a policy; a policy is
/// consulted only for an operation that is already dangerous; a policy that
/// cannot be honored is an operator error, never a silent downgrade to "no
/// policy". Any refusal names its remedy and returns without connecting.
///
/// `Box<dyn Error>` is part of the signature only so that the caller can use
/// the same `?`-based `run_with` it always has; this function never returns
/// one.
pub(crate) fn admit(
    name: &str,
    kind: &str,
    options: &Options,
) -> Result<Gate, Box<dyn std::error::Error>> {
    let risk = risk_of_kind(kind);
    let now_ms = current_time_ms();
    // A policy is consulted ONLY for an operation that is already dangerous
    // and that `--yes` did not already authorize. Two consequences are
    // deliberate: a read never touches the policy file, so a broken policy
    // cannot brick `health`; and `--yes` is a complete authorization on its
    // own, so a policy cannot block an explicitly authorized run.
    let mut policy_error: Option<(PathBuf, PolicyError)> = None;
    let mut policy_expired: Option<(PathBuf, u64)> = None;
    let mut policy_grant = false;
    if risk.requires_authorization() && !options.yes {
        if let Some(path) = resolved_policy_path(options) {
            match load_policy(&path) {
                Ok(policy) => {
                    policy_grant = policy.authorizes(kind, now_ms);
                    // Report a lapse only when the policy actually named this
                    // kind. An expired policy that never mentioned `kind` must
                    // not be described as a lapsed grant *for* it, or the
                    // refusal asserts something about the file that is not
                    // true.
                    if !policy_grant && policy.authorize.contains(kind) {
                        policy_expired = policy.expired_at(now_ms).map(|expiry| (path, expiry));
                    }
                }
                Err(error) => policy_error = Some((path, error)),
            }
        }
    }
    // A policy that cannot be honored is an operator error, not a missing
    // authorization: it is reported as such and nothing is sent. Silently
    // treating it as "no policy" would let a typo quietly downgrade a
    // pre-authorization the operator believes is in force.
    if let Some((path, error)) = policy_error {
        let message = format!("cannot use the policy {}: {error}", path.display());
        if options.json {
            println!(
                "{}",
                error_envelope(
                    name,
                    &minted_command_id(options, name),
                    "policy_invalid",
                    &message
                )
            );
            return Ok(Gate::Refused(EXIT_USAGE));
        }
        eprintln!("symbiote: {message}");
        eprintln!("         no request was sent (exit 1)");
        return Ok(Gate::Refused(EXIT_USAGE));
    }
    let decision = authorize(
        risk,
        options.yes,
        policy_grant,
        std::io::stdin().is_terminal(),
    );
    // Decided BEFORE anything touches the daemon: an unauthorized invocation
    // must not even connect (pinned by the integration test's never-accepted
    // socket).
    match decision {
        Authorization::NotRequired | Authorization::Flag | Authorization::Policy => {}
        Authorization::Prompt => {
            if !confirm(name, kind) {
                if options.json {
                    println!(
                        "{}",
                        error_envelope(
                            name,
                            &minted_command_id(options, name),
                            "authorization_required",
                            "interactive confirmation was not given"
                        )
                    );
                } else {
                    eprintln!(
                        "symbiote: {name} ({kind}) was not confirmed; no request was sent (exit 3)"
                    );
                }
                return Ok(Gate::Refused(EXIT_AUTHORIZATION_REQUIRED));
            }
        }
        Authorization::Refused => {
            // The refusal names every remedy, and names the policy explicitly
            // when one was configured but had lapsed — otherwise an operator
            // whose policy expired sees only "--yes" and never learns why.
            let message = match &policy_expired {
                Some((path, expiry)) => format!(
                    "the policy {} pre-authorized {kind} but expired at {expiry}",
                    path.display()
                ),
                None => format!("noninteractive runs require --yes or a policy naming \"{kind}\""),
            };
            if options.json {
                println!(
                    "{}",
                    error_envelope(
                        name,
                        &minted_command_id(options, name),
                        "authorization_required",
                        &message
                    )
                );
            } else {
                eprintln!("symbiote: {name} sends the dangerous operation {kind}; {message}");
                if policy_expired.is_none() {
                    eprintln!("         (an interactive run prompts instead; no request was sent)");
                }
            }
            return Ok(Gate::Refused(EXIT_AUTHORIZATION_REQUIRED));
        }
    }
    Ok(Gate::Proceed)
}
