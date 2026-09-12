//! The production transport for the pinned Codex App Server: versionless
//! JSONL framing over a sandboxed process. The framing logic lives in
//! [`CodexServerProcess`], generic over [`FrameIo`] so unit tests can drive
//! it against a real subprocess through the shared `JsonlTransport` without
//! bubblewrap, while the composed launcher ([`launch_sandboxed`]) runs the
//! pinned binary inside the Host sandbox with a disposable HOME, no network
//! and a read-only or worktree-writable reserved worktree (#211/#218).
//!
//! The transport owns exactly the wire facts: request-id assignment, frame
//! classification (response / notification / server request), bounded
//! buffering, strict envelope shapes, and the per-method denial bodies from
//! the pinned response schemas. It grants nothing: there is no code path
//! that sends an approval, an account operation or a credential.
#[cfg(test)]
use crate::CodexTransport;
use crate::{ApprovalDecision, DriverError, ServerRequest};
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;
use symbiote_domain::{HostId, RootId, Timestamp};
use symbiote_runtime_transport::{TransportError, TransportLimits};

/// Frames are bounded and duplicate-key-rejected by the shared transport.
const MAX_BUFFERED_NOTIFICATIONS: usize = 1024;
const MAX_BUFFERED_SERVER_REQUESTS: usize = 256;
const MAX_METHOD_BYTES: usize = 256;
const DEFAULT_FRAME_TIMEOUT: Duration = Duration::from_millis(250);
/// How long `call` waits for its correlated response before failing.
const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a cancel waits for the process to die before giving up.
const CANCEL_TIMEOUT: Duration = Duration::from_millis(500);

/// The raw framed IO both transports provide: send one JSON value, receive
/// one JSON value or `None` when nothing arrives within the timeout. `Err`
/// means the stream is closed or failed (process exit, broken pipe, EOF).
pub trait FrameIo {
    fn send_frame(&mut self, frame: &Value, timeout: Duration) -> Result<(), DriverError>;
    fn recv_frame(&mut self, timeout: Duration) -> Result<Option<Value>, DriverError>;
    /// Terminates the process this IO owns, if any, so a run that stops at its
    /// deadline does not leave a harness behind. IO with nothing to stop is a
    /// no-op.
    fn cancel_frame_io(&mut self) {}
}

impl FrameIo for symbiote_sandbox::SandboxProcess {
    fn send_frame(&mut self, frame: &Value, timeout: Duration) -> Result<(), DriverError> {
        self.send(frame, timeout)
            .map_err(|_| DriverError::TransportFailed)
    }
    fn recv_frame(&mut self, timeout: Duration) -> Result<Option<Value>, DriverError> {
        match self.recv(timeout) {
            Ok(frame) => Ok(Some(frame)),
            Err(symbiote_sandbox::SandboxError::Transport(TransportError::DeadlineExceeded)) => {
                Ok(None)
            }
            Err(_) => Err(DriverError::TransportFailed),
        }
    }
    fn cancel_frame_io(&mut self) {
        let _ = self.cancel(CANCEL_TIMEOUT);
    }
}

impl FrameIo for symbiote_runtime_transport::JsonlTransport {
    fn send_frame(&mut self, frame: &Value, timeout: Duration) -> Result<(), DriverError> {
        self.send(frame, timeout)
            .map_err(|_| DriverError::TransportFailed)
    }
    fn recv_frame(&mut self, timeout: Duration) -> Result<Option<Value>, DriverError> {
        match self.recv(timeout) {
            Ok(frame) => Ok(Some(frame)),
            Err(TransportError::DeadlineExceeded) => Ok(None),
            Err(_) => Err(DriverError::TransportFailed),
        }
    }
    fn cancel_frame_io(&mut self) {
        let _ = self.cancel(CANCEL_TIMEOUT);
    }
}

/// The request envelope the transport sends. Versionless on purpose: the
/// pinned App Server rejects `jsonrpc`-carrying envelopes, and #483's probe
/// proved this exact shape against the installed binary.
pub fn request_envelope(id: u64, method: &str, params: &Value) -> Value {
    json!({"id": id, "method": method, "params": params})
}

pub fn notification_envelope(method: &str, params: &Value) -> Value {
    json!({"method": method, "params": params})
}

/// Classifies one inbound frame. Mirrors the pinned protocol's three frame
/// kinds; anything else is refused rather than guessed.
enum Inbound {
    Response {
        id: Value,
        result: Option<Value>,
        error: bool,
    },
    Notification(Value),
    ServerRequest(ServerRequest),
}

fn classify(frame: Value) -> Result<Inbound, DriverError> {
    let Some(object) = frame.as_object() else {
        return Err(DriverError::MalformedFrame);
    };
    for key in object.keys() {
        if !matches!(
            key.as_str(),
            "id" | "method" | "params" | "result" | "error"
        ) {
            return Err(DriverError::MalformedFrame);
        }
    }
    match (object.get("method"), object.get("id")) {
        (Some(method), None) => {
            let method = method.as_str().ok_or(DriverError::MalformedFrame)?;
            if method.is_empty() || method.len() > MAX_METHOD_BYTES {
                return Err(DriverError::MalformedFrame);
            }
            if object.keys().any(|k| k != "method" && k != "params") {
                return Err(DriverError::MalformedFrame);
            }
            Ok(Inbound::Notification(json!({
                "method": method,
                "params": object.get("params").cloned().unwrap_or_else(|| json!({})),
            })))
        }
        (Some(method), Some(id)) => {
            let method = method.as_str().ok_or(DriverError::MalformedFrame)?;
            if method.is_empty() || method.len() > MAX_METHOD_BYTES {
                return Err(DriverError::MalformedFrame);
            }
            if object
                .keys()
                .any(|k| k != "method" && k != "params" && k != "id")
            {
                return Err(DriverError::MalformedFrame);
            }
            // Ids are echoed verbatim (string or number); null is not an id.
            if id.is_null() {
                return Err(DriverError::MalformedFrame);
            }
            Ok(Inbound::ServerRequest(ServerRequest {
                id: id.clone(),
                method: method.to_owned(),
                params: object.get("params").cloned().unwrap_or_else(|| json!({})),
            }))
        }
        (None, Some(id)) => {
            if id.is_null() {
                return Err(DriverError::MalformedFrame);
            }
            // A response carries exactly id + result xor id + error; any
            // other key makes the frame out-of-shape even if the key is
            // individually known to the protocol.
            if object
                .keys()
                .any(|k| k != "id" && k != "result" && k != "error")
            {
                return Err(DriverError::MalformedFrame);
            }
            match (object.get("result"), object.get("error")) {
                (Some(result), None) => Ok(Inbound::Response {
                    id: id.clone(),
                    result: Some(result.clone()),
                    error: false,
                }),
                (None, Some(_)) => Ok(Inbound::Response {
                    id: id.clone(),
                    result: None,
                    error: true,
                }),
                _ => Err(DriverError::MalformedFrame),
            }
        }
        (None, None) => Err(DriverError::MalformedFrame),
    }
}

/// The per-method denial body from the pinned response schemas. `None` means
/// the pinned protocol defines no safe denial for this method and the driver
/// leaves it unanswered. Every body here is a refusal: no variant grants.
pub fn denial_body(method: &str) -> Option<Value> {
    let body = match method {
        // Legacy approvals answer with ReviewDecision "denied".
        "execCommandApproval" | "applyPatchApproval" => json!({"decision": "denied"}),
        // Current command/file approvals answer with "decline".
        "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
            json!({"decision": "decline"})
        }
        // An empty granted profile grants nothing.
        "item/permissions/requestApproval" => json!({"permissions": {}, "scope": "turn"}),
        // Elicitation is declined.
        "mcpServer/elicitation/request" => json!({"action": "decline"}),
        // A client-executed dynamic tool call is refused with failure.
        "item/tool/call" => json!({"success": false, "contentItems": []}),
        _ => return None,
    };
    Some(body)
}

/// The framed App Server process. One outstanding client request at a time;
/// notifications and server requests that interleave a `call` are buffered
/// (bounded) and surfaced in arrival order.
pub struct CodexServerProcess<T: FrameIo> {
    io: T,
    notifications: VecDeque<Value>,
    server_requests: VecDeque<ServerRequest>,
    next_request_id: u64,
    frame_timeout: Duration,
    call_timeout: Duration,
    /// The dispatch's declared wall time, once the driver hands it over: no
    /// I/O wait may outlive it. `None` means only the transport's own timeouts
    /// apply — the case for a transport the driver never bounded.
    deadline: Option<std::time::Instant>,
}

impl<T: FrameIo> CodexServerProcess<T> {
    pub fn new(io: T) -> Self {
        Self {
            io,
            notifications: VecDeque::new(),
            server_requests: VecDeque::new(),
            next_request_id: 1,
            frame_timeout: DEFAULT_FRAME_TIMEOUT,
            call_timeout: DEFAULT_CALL_TIMEOUT,
            deadline: None,
        }
    }

    pub fn io(&mut self) -> &mut T {
        &mut self.io
    }

    /// Bounds every later I/O wait by the dispatch's declared wall time. The
    /// driver owns the clock and hands over an absolute deadline; `None`
    /// clears it.
    pub fn bind_deadline(&mut self, deadline: Option<std::time::Instant>) {
        self.deadline = deadline;
    }

    /// The budget an I/O wait may use: its own, or the time left before the
    /// declared wall time, whichever ends first. `None` means the deadline is
    /// already spent, so nothing may be waited for.
    fn io_budget(&self, budget: Duration) -> Option<Duration> {
        match self.deadline {
            Some(deadline) => deadline
                .checked_duration_since(std::time::Instant::now())
                .filter(|left| !left.is_zero())
                .map(|left| left.min(budget)),
            None => Some(budget),
        }
    }

    /// Why a wait ended with no budget left: the declared wall time when that
    /// is what expired, the transport's own timeout otherwise.
    fn exhausted(&self) -> DriverError {
        match self.deadline {
            Some(deadline) if deadline <= std::time::Instant::now() => {
                DriverError::WallTimeExceeded
            }
            _ => DriverError::TransportFailed,
        }
    }

    /// Buffers one non-response inbound frame. A response outside an
    /// outstanding `call` is a protocol violation, never buffered.
    fn buffer(&mut self, inbound: Inbound) -> Result<(), DriverError> {
        match inbound {
            Inbound::Notification(notification) => {
                if self.notifications.len() >= MAX_BUFFERED_NOTIFICATIONS {
                    return Err(DriverError::MalformedFrame);
                }
                self.notifications.push_back(notification);
                Ok(())
            }
            Inbound::ServerRequest(request) => {
                if self.server_requests.len() >= MAX_BUFFERED_SERVER_REQUESTS {
                    return Err(DriverError::MalformedFrame);
                }
                self.server_requests.push_back(request);
                Ok(())
            }
            Inbound::Response { .. } => Err(DriverError::UnexpectedResponse),
        }
    }

    fn recv_until_notification(&mut self) -> Result<Option<Value>, DriverError> {
        if let Some(notification) = self.notifications.pop_front() {
            return Ok(Some(notification));
        }
        // A poll window never outlives the declared wall time: once it is
        // spent, the next read waits for nothing and the driver's own deadline
        // check stops the turn.
        let Some(window) = self.io_budget(self.frame_timeout) else {
            return Ok(None);
        };
        let deadline = std::time::Instant::now()
            .checked_add(window)
            .ok_or(DriverError::TransportFailed)?;
        loop {
            let Some(remaining) = deadline
                .checked_duration_since(std::time::Instant::now())
                .filter(|d| !d.is_zero())
            else {
                // Poll window over. Buffered frames surface through the
                // queue on the next call; this is not a transport failure.
                return Ok(None);
            };
            match self.io.recv_frame(remaining)? {
                Some(frame) => match classify(frame)? {
                    Inbound::Notification(notification) => return Ok(Some(notification)),
                    other => self.buffer(other)?,
                },
                None => return Ok(None),
            }
        }
    }
}

/// The full refusal response envelope: the server request's id echoed
/// verbatim with the per-method denial body. `None` when the pinned schemas
/// define no safe denial body for the method (the driver leaves it
/// unanswered and never calls the transport for it).
pub fn refusal_envelope(request: &ServerRequest) -> Option<Value> {
    denial_body(&request.method).map(|body| json!({"id": request.id, "result": body}))
}

impl<T: FrameIo> crate::CodexTransport for CodexServerProcess<T> {
    fn call(&mut self, method: &str, params: &Value) -> Result<Value, DriverError> {
        if method.is_empty() || method.len() > MAX_METHOD_BYTES {
            return Err(DriverError::InvalidInput);
        }
        let send_budget = self
            .io_budget(self.call_timeout)
            .ok_or_else(|| self.exhausted())?;
        let id = self.next_request_id;
        if id == u64::MAX {
            return Err(DriverError::TransportFailed);
        }
        self.next_request_id += 1;
        self.io
            .send_frame(&request_envelope(id, method, params), send_budget)?;
        // The response wait ends at whichever comes first: the call's own
        // timeout or the dispatch's declared wall time. Which one expired
        // decides the typed error, so a harness that never answers is reported
        // as out of wall time rather than as a broken transport.
        let call_end = std::time::Instant::now()
            .checked_add(self.call_timeout)
            .ok_or(DriverError::TransportFailed)?;
        let (end, capped_by_wall_time) = match self.deadline {
            Some(deadline) if deadline <= call_end => (deadline, true),
            _ => (call_end, false),
        };
        let expiry = if capped_by_wall_time {
            DriverError::WallTimeExceeded
        } else {
            DriverError::TransportFailed
        };
        loop {
            let remaining = end
                .checked_duration_since(std::time::Instant::now())
                .filter(|left| !left.is_zero());
            let Some(remaining) = remaining else {
                return Err(expiry);
            };
            let frame = self.io.recv_frame(remaining)?.ok_or(expiry)?;
            match classify(frame)? {
                Inbound::Response {
                    id: frame_id,
                    result,
                    error,
                } => {
                    if frame_id.as_u64() != Some(id) {
                        return Err(DriverError::UnexpectedResponse);
                    }
                    if error {
                        return Err(DriverError::RpcFailure);
                    }
                    return result.ok_or(DriverError::MalformedFrame);
                }
                other => {
                    self.buffer(other)?;
                }
            }
        }
    }

    fn notify(&mut self, method: &str, params: &Value) -> Result<(), DriverError> {
        if method.is_empty() || method.len() > MAX_METHOD_BYTES {
            return Err(DriverError::InvalidInput);
        }
        let budget = self
            .io_budget(self.call_timeout)
            .ok_or_else(|| self.exhausted())?;
        self.io
            .send_frame(&notification_envelope(method, params), budget)
    }

    fn set_deadline(&mut self, deadline: Option<std::time::Instant>) {
        self.bind_deadline(deadline);
    }

    fn cancel(&mut self) {
        self.io.cancel_frame_io();
    }

    fn recv_notification(&mut self) -> Result<Option<Value>, DriverError> {
        self.recv_until_notification()
    }

    fn recv_server_request(&mut self) -> Result<Option<ServerRequest>, DriverError> {
        Ok(self.server_requests.pop_front())
    }

    fn refuse_server_request(
        &mut self,
        request: &ServerRequest,
        decision: ApprovalDecision,
    ) -> Result<(), DriverError> {
        // The only decision the driver may send: a per-method denial.
        // Aborted and Unanswered are recorded by the driver; the transport
        // sends nothing for them and must not be asked to.
        if decision != ApprovalDecision::Denied {
            return Err(DriverError::InvalidInput);
        }
        let envelope = refusal_envelope(request).ok_or(DriverError::InvalidInput)?;
        let budget = self
            .io_budget(self.call_timeout)
            .ok_or_else(|| self.exhausted())?;
        self.io.send_frame(&envelope, budget)
    }
}

/// The command fingerprint for the pinned launch, matching the arguments
/// [`launch_sandboxed`] uses. The caller must build their `ResourceSnapshot`
/// (and the consent embedding it) with this fingerprint — the sandbox
/// recomputes and compares it and refuses on any mismatch.
pub fn codex_launch_fingerprint(
    root_id: &RootId,
    worktree: &Path,
    profile: symbiote_sandbox::Profile,
) -> Result<symbiote_trust::Fingerprint, DriverError> {
    symbiote_sandbox::fingerprint_command(
        root_id,
        worktree,
        profile,
        "/usr/bin/codex",
        &[
            "app-server".to_owned(),
            "--listen".to_owned(),
            "stdio://".to_owned(),
        ],
    )
    .map_err(|_| DriverError::InvalidContract)
}

/// Composed launch of the pinned binary inside the Host sandbox. The
/// arguments and premises mirror the #483 discovery proof exactly: trusted
/// helper, consented snapshot whose fingerprint covers the codex invocation,
/// `/usr/bin/codex app-server --listen stdio://`, disposable HOME, isolated
/// network. The caller supplies the reserved worktree; no credential
/// directory is ever mounted.
#[allow(clippy::too_many_arguments)]
pub fn launch_sandboxed(
    helper_path: &Path,
    consent: &symbiote_trust::ResourceConsent,
    policy: &symbiote_domain::AccessSnapshot,
    host: &HostId,
    at: Timestamp,
    root_id: &RootId,
    worktree: &Path,
    protected_paths: &[std::path::PathBuf],
    profile: symbiote_sandbox::Profile,
    limits: TransportLimits,
    // The address-space ceiling the launched harness runs under: the declared
    // memory bound of the dispatch this launch serves.
    memory_limit_bytes: u64,
) -> Result<CodexServerProcess<symbiote_sandbox::SandboxProcess>, DriverError> {
    let process = symbiote_sandbox::launch(symbiote_sandbox::LaunchRequest {
        helper_path,
        consent,
        snapshot: &consent.snapshot,
        policy,
        host,
        at,
        root_id,
        worktree,
        protected_paths,
        profile,
        program: "/usr/bin/codex",
        args: &[
            "app-server".to_owned(),
            "--listen".to_owned(),
            "stdio://".to_owned(),
        ],
        limits,
        address_space_bytes: memory_limit_bytes,
    })
    .map_err(|_| DriverError::TransportFailed)?;
    Ok(CodexServerProcess::new(process))
}

/// Spawns an arbitrary process for transport framing tests: real subprocess,
/// real pipes, no sandbox claims. Not a harness integration.
#[cfg(test)]
pub(crate) fn spawn_test_server(
    script: &str,
) -> CodexServerProcess<symbiote_runtime_transport::JsonlTransport> {
    use std::collections::BTreeMap;
    use symbiote_runtime_transport::SpawnSpec;
    let transport = symbiote_runtime_transport::JsonlTransport::spawn(
        SpawnSpec {
            executable: "/usr/bin/sh".into(),
            args: vec!["-c".into(), script.into()],
            cwd: std::env::temp_dir(),
            env: BTreeMap::new(),
        },
        TransportLimits::default(),
    )
    .expect("test server spawns");
    CodexServerProcess::new(transport)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelopes_are_versionless_with_no_jsonrpc_field() {
        let request = request_envelope(7, "thread/start", &json!({"cwd": "/w"}));
        assert_eq!(
            request,
            json!({"id": 7, "method": "thread/start", "params": {"cwd": "/w"}})
        );
        assert!(request.get("jsonrpc").is_none());
        let notification = notification_envelope("initialized", &json!({}));
        assert_eq!(notification, json!({"method": "initialized", "params": {}}));
        assert!(notification.get("id").is_none());
        assert!(notification.get("jsonrpc").is_none());
    }

    #[test]
    fn denial_bodies_match_the_pinned_response_schemas() {
        assert_eq!(
            denial_body("execCommandApproval"),
            Some(json!({"decision": "denied"}))
        );
        assert_eq!(
            denial_body("applyPatchApproval"),
            Some(json!({"decision": "denied"}))
        );
        assert_eq!(
            denial_body("item/commandExecution/requestApproval"),
            Some(json!({"decision": "decline"}))
        );
        assert_eq!(
            denial_body("item/fileChange/requestApproval"),
            Some(json!({"decision": "decline"}))
        );
        assert_eq!(
            denial_body("item/permissions/requestApproval"),
            Some(json!({"permissions": {}, "scope": "turn"}))
        );
        assert_eq!(
            denial_body("mcpServer/elicitation/request"),
            Some(json!({"action": "decline"}))
        );
        assert_eq!(
            denial_body("item/tool/call"),
            Some(json!({"success": false, "contentItems": []}))
        );
        // No safe denial body exists for these; they stay unanswered.
        assert_eq!(denial_body("item/tool/requestUserInput"), None);
        assert_eq!(denial_body("account/chatgptAuthTokens/refresh"), None);
        assert_eq!(denial_body("totally/unknown"), None);
        // No body anywhere grants anything.
        for method in [
            "execCommandApproval",
            "applyPatchApproval",
            "item/commandExecution/requestApproval",
            "item/fileChange/requestApproval",
            "item/permissions/requestApproval",
            "mcpServer/elicitation/request",
            "item/tool/call",
        ] {
            let body = denial_body(method).unwrap().to_string();
            assert!(!body.contains("accept"));
            assert!(!body.contains("approved"));
            assert!(!body.contains("\"success\":true"));
        }
    }

    #[test]
    fn refusal_envelopes_echo_the_server_request_id_verbatim() {
        for id in [json!("server-1"), json!(42)] {
            let request = ServerRequest {
                id: id.clone(),
                method: "execCommandApproval".into(),
                params: json!({"callId": "c1"}),
            };
            let envelope = refusal_envelope(&request).unwrap();
            assert_eq!(envelope["id"], id);
            assert_eq!(envelope["result"], json!({"decision": "denied"}));
            assert!(envelope.get("method").is_none());
        }
        // Methods without a pinned denial body produce no envelope at all.
        let request = ServerRequest {
            id: json!(1),
            method: "item/tool/requestUserInput".into(),
            params: json!({}),
        };
        assert!(refusal_envelope(&request).is_none());
    }

    #[test]
    fn call_correlates_responses_and_buffers_interleaved_frames() {
        // A real subprocess emits a notification and a server request
        // between our two requests and their responses.
        let mut server = spawn_test_server(
            "printf '%s\\n' \
             '{\"id\":1,\"result\":{\"userAgent\":\"symbiote/0.118.0\"}}' \
             '{\"method\":\"thread/status/changed\",\"params\":{\"threadId\":\"t\"}}' \
             '{\"id\":\"srv-9\",\"method\":\"execCommandApproval\",\"params\":{\"callId\":\"c\"}}' \
             '{\"id\":2,\"result\":{\"thread\":{\"id\":\"thr\"}}}'; sleep 30",
        );
        let result = server.call("initialize", &json!({})).unwrap();
        assert_eq!(result["userAgent"], "symbiote/0.118.0");
        // The interleaved notification surfaces next...
        let notification = server.recv_notification().unwrap().unwrap();
        assert_eq!(notification["method"], "thread/status/changed");
        // The second call pumps through the interleaved server request (it
        // is buffered, not dropped) and consumes its own response.
        let thread = server.call("thread/start", &json!({})).unwrap();
        assert_eq!(thread["thread"]["id"], "thr");
        // The buffered server request now surfaces with its id verbatim.
        let request = server.recv_server_request().unwrap().unwrap();
        assert_eq!(request.method, "execCommandApproval");
        assert_eq!(request.id, json!("srv-9"));
        // Then the stream is quiet: a poll returns None, not an error.
        assert!(server.recv_notification().unwrap().is_none());
    }

    #[test]
    fn stray_and_malformed_frames_are_rejected() {
        // Response with the wrong id is rejected even mid-call.
        let mut server = spawn_test_server("printf '%s\\n' '{\"id\":99,\"result\":{}}'; sleep 30");
        assert_eq!(
            server.call("initialize", &json!({})),
            Err(DriverError::UnexpectedResponse)
        );
        // Frames outside a call that look like responses are violations.
        let mut server = spawn_test_server("printf '%s\\n' '{\"id\":1,\"result\":{}}'; sleep 30");
        assert_eq!(
            server.recv_notification(),
            Err(DriverError::UnexpectedResponse)
        );
        // Unknown envelope fields are rejected.
        let mut server = spawn_test_server(
            "printf '%s\\n' '{\"method\":\"m\",\"params\":{},\"extra\":1}'; sleep 30",
        );
        assert_eq!(server.recv_notification(), Err(DriverError::MalformedFrame));
    }

    #[test]
    fn rpc_error_responses_fail_the_call() {
        let mut server = spawn_test_server(
            "printf '%s\\n' '{\"id\":1,\"error\":{\"code\":-32000,\"message\":\"no\"}}'; sleep 30",
        );
        assert_eq!(
            server.call("thread/start", &json!({})),
            Err(DriverError::RpcFailure)
        );
    }

    #[test]
    fn process_exit_is_a_hard_failure_not_silence() {
        let mut server = spawn_test_server("exit 0");
        // Give the process a moment to exit, then the stream must report a
        // closed transport rather than an indefinite silent poll.
        let mut waited = 0;
        loop {
            match server.recv_notification() {
                Err(DriverError::TransportFailed) => break,
                Ok(None) => {
                    waited += 1;
                    assert!(waited < 200, "silent poll never resolved to closed");
                }
                other => panic!("unexpected poll result: {other:?}"),
            }
        }
    }
}
