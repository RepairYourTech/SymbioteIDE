//! External Symbiote Agent worker loop: drives the pinned Codex App Server
//! (`codex-cli 0.118.0`) as an external harness over versionless JSONL RPC.
//! This crate owns the external side of #465. It is deliberately
//! offline-testable: the [`CodexTransport`] trait abstracts the framed IPC so
//! the driver, its state machine, its event journaling and its completion
//! handoff are exercised against deterministic fixture servers without the
//! binary, network access, credential reads or spending. The concrete
//! transport is the shared `JsonlTransport` in `symbiote-runtime-transport`
//! plus the sandbox launcher; the `codex_discover` proof (#483) established
//! this pinned binary's offline protocol compatibility, and this crate
//! extends the same versionless envelope (no `jsonrpc` field, string-or-number
//! ids, notifications interleaved with responses).
//!
//! Boundary: the harness is untrusted execution. It never writes canonical
//! state. The driver emits typed [`RuntimeEvent`]s carrying the session
//! binding with `RuntimeKind::ExternalHarness` and produces a completion
//! report the Host routes through RequestCompletion → BeginVerification →
//! Complete with independent review. Every server-initiated approval request
//! (`execCommandApproval`, `applyPatchApproval`, `item/permissions/requestApproval`,
//! `item/tool/requestUserInput`, `mcpServer/elicitation/request`) is refused
//! and recorded as an [`ApprovalRefusal`]: the dispatch contract's access
//! snapshot is the only permission authority, and granting the harness's
//! runtime escalation requests from inside the worker loop would bypass it.
//! Steering a live turn is an explicit [`ControlOutcome`]; the driver never
//! guesses whether the harness accepted an interrupt.
//!
//! Credential separation: the harness authenticates with its own configured
//! account. The driver sends no credentials, no API keys and no native
//! billing route; `account/read` is never called here (discovery already
//! proved the handshake) and `refreshToken` is never sent. A native provider
//! credential reference is never copied into an external session.
use symbiote_domain::*;
use symbiote_runtime_sdk::events::{
    EventText, ObservationKind, RuntimeEvent, RuntimeEventKind, SessionBinding,
};

pub mod process;

pub const EXTERNAL_LOOP_VERSION: u32 = 1;
/// One observed turn may absorb at most
/// `MAX_TURN_NOTIFICATION_FRAMES` harness frames before the driver gives up
/// rather than journaling unboundedly. (The transport separately bounds how
/// many frames one `call` may buffer: 1024.)
pub const MAX_TURN_NOTIFICATION_FRAMES: usize = 1024;
/// Empty polls while waiting for harness frames. The production transport
/// polls at 250 ms, so this bounds a silent-but-alive harness (a long model
/// turn) at roughly 17 minutes before declaring the transport lost. Frames
/// that surfaced server requests do not count as silence: the harness was
/// alive and conversed with.
pub const MAX_EMPTY_NOTIFICATION_POLLS: usize = 4096;
/// Notification text (agent messages, command output) is bounded by the SDK
/// event contract; oversized text truncates on char boundaries with a marker.
pub const MAX_EVENT_TEXT_BYTES: usize = symbiote_runtime_sdk::events::MAX_EVENT_TEXT_BYTES;
/// Completion reports are bounded by the same limit as event text.
pub const MAX_REPORT_BYTES: usize = MAX_EVENT_TEXT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverError {
    /// The dispatch contract no longer validates (expired enforcement, etc.).
    InvalidContract,
    /// The dispatch is native; the external driver refuses to run it.
    NotExternalHarness,
    /// Caller input failed local bounds.
    InvalidInput,
    /// The transport failed or the process exited mid-call.
    TransportFailed,
    /// The harness produced a frame the pinned protocol does not allow.
    MalformedFrame,
    /// A response correlated to a different request than the outstanding one.
    UnexpectedResponse,
    /// The harness answered a request with a JSON-RPC error object.
    RpcFailure,
    /// The server did not report the pinned App Server version.
    UnsupportedVersion,
    /// The harness answered the launch handshake with an unexpected state.
    ContractMismatch,
    /// Completion was already filed; the driver is finished.
    AlreadyComplete,
}

impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "external driver: {self:?}")
    }
}
impl std::error::Error for DriverError {}

/// One server-initiated request frame awaiting an answer. The `id` is kept
/// verbatim (string or number) because the refusal response must echo it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerRequest {
    pub id: serde_json::Value,
    pub method: String,
    pub params: serde_json::Value,
}

/// The framed IPC boundary to the pinned App Server. Implementations translate
/// a versionless request envelope into either a correlated response result or
/// an error; notifications are surfaced separately. The production
/// implementation wraps the sandboxed `JsonlTransport` process (see the
/// `process` module); tests provide deterministic fixture servers.
pub trait CodexTransport {
    /// Send a request envelope (no `jsonrpc` field, number ids assigned by
    /// the transport) and return the matching response's `result` object.
    /// Interleaved server notifications and requests are buffered and
    /// surfaced through [`Self::recv_notification`] and
    /// [`Self::recv_server_request`].
    fn call(
        &mut self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, DriverError>;
    /// Send a notification envelope (no id, no response expected).
    fn notify(&mut self, method: &str, params: &serde_json::Value) -> Result<(), DriverError>;
    /// Receive one server-initiated notification frame. `Ok(None)` means
    /// nothing arrived within the transport's poll window — keep polling;
    /// `Err` means the stream is closed or failed.
    fn recv_notification(&mut self) -> Result<Option<serde_json::Value>, DriverError>;
    /// Receive one buffered server-initiated request frame, or `None` when
    /// none is pending.
    fn recv_server_request(&mut self) -> Result<Option<ServerRequest>, DriverError>;
    /// Refuse one pending server request by echoing its id with a per-method
    /// denial body from the pinned response schemas. `Unanswered` refusals
    /// are never sent by the driver; implementations must not invent bodies.
    fn refuse_server_request(
        &mut self,
        request: &ServerRequest,
        decision: ApprovalDecision,
    ) -> Result<(), DriverError>;
}

/// Lets a Host hold `Box<dyn CodexTransport>` factories and pass
/// `&mut dyn CodexTransport` into the driver.
impl<T: CodexTransport + ?Sized> CodexTransport for &mut T {
    fn call(
        &mut self,
        method: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, DriverError> {
        (**self).call(method, params)
    }
    fn notify(&mut self, method: &str, params: &serde_json::Value) -> Result<(), DriverError> {
        (**self).notify(method, params)
    }
    fn recv_notification(&mut self) -> Result<Option<serde_json::Value>, DriverError> {
        (**self).recv_notification()
    }
    fn recv_server_request(&mut self) -> Result<Option<ServerRequest>, DriverError> {
        (**self).recv_server_request()
    }
    fn refuse_server_request(
        &mut self,
        request: &ServerRequest,
        decision: ApprovalDecision,
    ) -> Result<(), DriverError> {
        (**self).refuse_server_request(request, decision)
    }
}

/// How the driver disposed of one harness escalation request. Every refusal
/// is recorded: the model text never gains permissions through the harness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// The request was denied and the harness was told to continue.
    Denied,
    /// The request was denied and the harness was told to stop the turn.
    Aborted,
    /// No reply was sent: the method is outside the pinned approval shapes,
    /// so no refusal body is claimed to be understood. Recorded, never an
    /// approval.
    Unanswered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalRefusal {
    /// `execCommandApproval` — legacy exec approval; denied with `denied`.
    ExecCommand,
    /// `applyPatchApproval` — legacy patch approval; denied with `denied`.
    ApplyPatch,
    /// `item/commandExecution/requestApproval` — denied with `decline`.
    CommandExecutionRequest,
    /// `item/fileChange/requestApproval` — denied with `decline`.
    FileChangeRequest,
    /// `item/permissions/requestApproval` — denied with an empty granted
    /// profile (grants nothing).
    PermissionsRequest,
    /// `mcpServer/elicitation/request` — declined with action `decline`.
    McpElicitation,
    /// `item/tool/call` — a client-executed dynamic tool call; refused with
    /// `success: false` and no content.
    ToolCall,
    /// `item/tool/requestUserInput` — no reply: the pinned answer shape must
    /// match the question count, and inventing answers is worse than silence.
    ToolUserInput,
    /// A server request method outside the pinned schema's known approvals.
    Unknown,
}

impl ApprovalRefusal {
    fn from_method(method: &str) -> Self {
        match method {
            "execCommandApproval" => Self::ExecCommand,
            "applyPatchApproval" => Self::ApplyPatch,
            "item/commandExecution/requestApproval" => Self::CommandExecutionRequest,
            "item/fileChange/requestApproval" => Self::FileChangeRequest,
            "item/permissions/requestApproval" => Self::PermissionsRequest,
            "mcpServer/elicitation/request" => Self::McpElicitation,
            "item/tool/call" => Self::ToolCall,
            "item/tool/requestUserInput" => Self::ToolUserInput,
            _ => Self::Unknown,
        }
    }
    /// Whether the pinned response schemas give this refusal a well-defined
    /// denial body the driver may send. Everything else stays unanswered.
    fn has_shaped_denial(self) -> bool {
        !matches!(self, Self::ToolUserInput | Self::Unknown)
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StopKind {
    /// The turn completed normally.
    Completed,
    /// The turn failed inside the harness (model/provider/sandbox error).
    Failed,
    /// The turn was interrupted by the harness itself.
    Interrupted,
    /// The driver stopped observing because the transport died.
    TransportLost,
}

/// Durable record of one external run: the harness identities and the
/// emitted events. The Host journals this; the driver holds no canonical
/// write authority.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ExternalRun {
    pub version: u32,
    pub dispatch_id: DispatchId,
    pub task_id: TaskId,
    pub harness_session_id: Option<String>,
    pub turns_observed: u32,
    pub approvals_refused: u32,
    pub usage_turns: u32,
    pub unreported_usage_turns: u32,
    pub completion_report: Option<String>,
    pub stopped: Option<StopKind>,
}

/// The external turn driver. Created per dispatch from already-authorized
/// state, exactly like the native loop: the dispatch contract supplies the
/// task identity, permissions ceiling and host; the driver adds nothing.
#[derive(Debug)]
pub struct ExternalSession {
    binding: SessionBinding,
    task_id: TaskId,
    events: Vec<RuntimeEvent>,
    next_event_sequence: u64,
    harness_thread_id: Option<String>,
    turns_observed: u32,
    approvals_refused: u32,
    usage_turns: u32,
    unreported_usage_turns: u32,
    completion_report: Option<String>,
    stopped: Option<StopKind>,
}

impl ExternalSession {
    /// The dispatch must still validate at `now`, and its runtime kind must
    /// be the external harness — a native dispatch is refused here rather
    /// than silently executed through the wrong loop.
    pub fn new(dispatch: &Dispatch, at: Timestamp) -> Result<Self, DriverError> {
        dispatch
            .contract()
            .validate_at(at)
            .map_err(|_| DriverError::InvalidContract)?;
        let contract = dispatch.contract();
        if contract.profile().runtime != RuntimeKind::ExternalHarness {
            return Err(DriverError::NotExternalHarness);
        }
        let session = SessionId::new(format!("sess_{}", dispatch.id().as_str()))
            .map_err(|_| DriverError::InvalidContract)?;
        Ok(Self {
            binding: SessionBinding {
                session_id: session,
                dispatch_id: dispatch.id().clone(),
                host_id: contract.host_id().clone(),
                runtime: RuntimeKind::ExternalHarness,
            },
            task_id: contract.task_id().clone(),
            events: Vec::new(),
            next_event_sequence: 1,
            harness_thread_id: None,
            turns_observed: 0,
            approvals_refused: 0,
            usage_turns: 0,
            unreported_usage_turns: 0,
            completion_report: None,
            stopped: None,
        })
    }

    pub fn binding(&self) -> &SessionBinding {
        &self.binding
    }

    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    pub fn harness_thread_id(&self) -> Option<&str> {
        self.harness_thread_id.as_deref()
    }

    pub fn run_summary(&self) -> ExternalRun {
        ExternalRun {
            version: EXTERNAL_LOOP_VERSION,
            dispatch_id: self.binding.dispatch_id.clone(),
            task_id: self.task_id.clone(),
            harness_session_id: self.harness_thread_id.clone(),
            turns_observed: self.turns_observed,
            approvals_refused: self.approvals_refused,
            usage_turns: self.usage_turns,
            unreported_usage_turns: self.unreported_usage_turns,
            completion_report: self.completion_report.clone(),
            stopped: self.stopped,
        }
    }

    /// Event ids stay within the 128-byte CommandId bound: the dispatch id
    /// contributes at most 118 ASCII bytes (128 minus the "sess_" prefix),
    /// so "x{seq}_" plus that suffix always fits for any legal session id.
    /// A failure to build a valid event is a hard error rather than a dropped
    /// observation.
    fn record(&mut self, payload: RuntimeEventKind) -> Result<(), DriverError> {
        if self.next_event_sequence == u64::MAX {
            return Err(DriverError::MalformedFrame);
        }
        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;
        // 3 ("x", digits) + 1 ('_') + suffix ≤ 128: keep a slice bound so the
        // format! output can never exceed the domain's CommandId charset/size
        // contract regardless of the sequence number's width.
        let session_suffix: String = self
            .binding
            .session_id
            .as_str()
            .chars()
            .take(118 - sequence.to_string().len().min(60))
            .collect();
        let id = CommandId::new(format!("x{sequence}_{session_suffix}"))
            .map_err(|_| DriverError::InvalidContract)?;
        match RuntimeEvent::new(id, sequence, self.binding.clone(), payload) {
            Ok(event) => {
                self.events.push(event);
                Ok(())
            }
            Err(_) => Err(DriverError::MalformedFrame),
        }
    }

    /// Starts a harness thread bound to the task worktree and files the task
    /// prompt as the first turn: `begin_thread` + [`Self::turn`].
    pub fn start_turn<T: CodexTransport + ?Sized>(
        &mut self,
        task_prompt: &str,
        worktree: &str,
        transport: &mut T,
    ) -> Result<(), DriverError> {
        if self.completion_report.is_some() || self.stopped.is_some() {
            return Err(DriverError::AlreadyComplete);
        }
        if task_prompt.trim().is_empty() || task_prompt.len() > 64 * 1024 {
            return Err(DriverError::InvalidInput);
        }
        self.begin_thread(worktree, transport)?;
        self.turn(task_prompt, transport)
    }

    /// Completes the pinned App Server handshake and starts the harness
    /// thread. `worktree_cwd` is the worktree path **as visible to the
    /// harness process**: through the sandboxed launcher the Host path is
    /// mounted at `/workspace` and invisible by its host name, so callers
    /// composing with [`process::launch_sandboxed`] pass `/workspace`; a
    /// caller owning the process directly passes its own authorized path.
    /// The harness's internal sandbox policy is pinned to read-only and its
    /// approval policy to never — the outer Host sandbox is the enforcement
    /// boundary. No model turn starts here; the `codex_thread_smoke` proof
    /// stops after this step.
    ///
    /// Once the handshake begins, the session is single-shot: any failure
    /// after the Ready event is terminal (the driver records a
    /// `TransportLost` stop and a diagnostic), because a retry would record
    /// a second Ready and make the journal unreplayable for the SDK's
    /// session tracker. Recovery is a fresh session with a fresh dispatch.
    pub fn begin_thread<T: CodexTransport + ?Sized>(
        &mut self,
        worktree_cwd: &str,
        transport: &mut T,
    ) -> Result<(), DriverError> {
        if self.completion_report.is_some() || self.stopped.is_some() {
            return Err(DriverError::AlreadyComplete);
        }
        if self.harness_thread_id.is_some() {
            return Err(DriverError::ContractMismatch);
        }
        if worktree_cwd.is_empty() || worktree_cwd.len() > 4096 || !worktree_cwd.starts_with('/') {
            return Err(DriverError::InvalidInput);
        }
        // The SDK's session consumers require Ready before any other event.
        self.record(RuntimeEventKind::Ready {})?;
        // From here the session has begun journaling: a failure must be
        // terminal so no entry point can append to a half-open session.
        if let Err(error) = self.complete_handshake(worktree_cwd, transport) {
            self.stopped = Some(StopKind::TransportLost);
            self.record(RuntimeEventKind::Diagnostic {
                message: truncate_event_text(&format!("harness handshake failed: {error}")),
            })?;
            return Err(error);
        }
        Ok(())
    }

    fn complete_handshake<T: CodexTransport + ?Sized>(
        &mut self,
        worktree_cwd: &str,
        transport: &mut T,
    ) -> Result<(), DriverError> {
        // Handshake first: initialize → pinned version check → initialized.
        // The pin mirrors the #483 discovery probe exactly; a server that
        // does not report the pinned version is refused before any thread,
        // turn or account interaction happens.
        let client_info = serde_json::json!({
            "clientInfo": {"name": "symbiote", "version": "0.1.0", "title": "Symbiote"}
        });
        let initialized = transport.call("initialize", &client_info)?;
        let user_agent = initialized
            .get("userAgent")
            .and_then(serde_json::Value::as_str)
            .ok_or(DriverError::MalformedFrame)?;
        let expected = format!(
            "symbiote/{}",
            symbiote_runtime_discovery::codex::CODEX_VERSION
        );
        if user_agent.len() > 1024 || user_agent.split_whitespace().next() != Some(&expected) {
            return Err(DriverError::UnsupportedVersion);
        }
        transport.notify("initialized", &serde_json::json!({}))?;
        // Thread identity is minted by the harness, not the driver. The
        // thread id is the correlation key for every later notification.
        let params = serde_json::json!({
            "cwd": worktree_cwd,
            "sandbox": "read-only",
            "approvalPolicy": "never",
        });
        let result = transport.call("thread/start", &params)?;
        let thread_id = result
            .get("thread")
            .and_then(|t| t.get("id"))
            .and_then(serde_json::Value::as_str)
            .ok_or(DriverError::MalformedFrame)?
            .to_owned();
        if !valid_correlation_id(&thread_id) {
            return Err(DriverError::MalformedFrame);
        }
        self.harness_thread_id = Some(thread_id);
        Ok(())
    }

    /// Files one turn and observes it to completion. All harness escalation
    /// requests surfaced while observing are refused; the dispatch's access
    /// snapshot is the only permission authority.
    pub fn turn<T: CodexTransport + ?Sized>(
        &mut self,
        prompt: &str,
        transport: &mut T,
    ) -> Result<(), DriverError> {
        if self.completion_report.is_some() || self.stopped.is_some() {
            return Err(DriverError::AlreadyComplete);
        }
        if prompt.trim().is_empty() || prompt.len() > 64 * 1024 {
            return Err(DriverError::InvalidInput);
        }
        let Some(thread_id) = self.harness_thread_id.clone() else {
            return Err(DriverError::ContractMismatch);
        };
        self.turns_observed += 1;
        let params = serde_json::json!({
            "threadId": thread_id,
            "input": [{"type": "text", "text": prompt}],
        });
        let result = match transport.call("turn/start", &params) {
            Ok(result) => result,
            Err(error) => {
                self.stopped = Some(StopKind::TransportLost);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!("harness transport failed: {error}")),
                })?;
                return Err(error);
            }
        };
        let turn_id = result
            .get("turn")
            .and_then(|t| t.get("id"))
            .and_then(serde_json::Value::as_str)
            .ok_or(DriverError::MalformedFrame)?
            .to_owned();
        if !valid_correlation_id(&turn_id) {
            return Err(DriverError::MalformedFrame);
        }
        self.observe_turn(&thread_id, &turn_id, transport)
    }

    /// Drains frames until the turn reaches a terminal status. Item payloads
    /// map to tracker-ingestible events; usage is accounted conservatively;
    /// every harness escalation request is refused and recorded. Once the
    /// terminal frame is seen, nothing further is absorbed: the stop is
    /// decided and later frames belong to no observed turn.
    fn observe_turn<T: CodexTransport + ?Sized>(
        &mut self,
        thread_id: &str,
        turn_id: &str,
        transport: &mut T,
    ) -> Result<(), DriverError> {
        let mut stop: Option<StopKind> = None;
        let mut frames = 0usize;
        let mut empty_polls = 0usize;
        while stop.is_none() {
            let notification = match transport.recv_notification() {
                Ok(Some(notification)) => {
                    empty_polls = 0;
                    Some(notification)
                }
                Ok(None) => {
                    // Nothing arrived within the poll window. A live harness
                    // may think for a long time; only a bounded number of
                    // silent polls reads as a lost transport.
                    empty_polls += 1;
                    if empty_polls > MAX_EMPTY_NOTIFICATION_POLLS {
                        self.stopped = Some(StopKind::TransportLost);
                        self.record(RuntimeEventKind::Diagnostic {
                            message: truncate_event_text("harness silent beyond the poll budget"),
                        })?;
                        return Err(DriverError::TransportFailed);
                    }
                    None
                }
                Err(error) => {
                    self.stopped = Some(StopKind::TransportLost);
                    self.record(RuntimeEventKind::Diagnostic {
                        message: truncate_event_text(&format!(
                            "harness closed before turn completion: {error}"
                        )),
                    })?;
                    return Err(error);
                }
            };
            // Server requests surface through the same frame pump as
            // notifications: drain everything that read surfaced before
            // processing the notification itself, so a harness blocked on a
            // pending approval is answered first.
            while let Some(request) = transport.recv_server_request()? {
                let refusal = ApprovalRefusal::from_method(&request.method);
                self.approvals_refused += 1;
                let decision = if refusal.has_shaped_denial() {
                    ApprovalDecision::Denied
                } else {
                    // No pinned denial body exists for this shape; no reply
                    // is invented. Recorded, never an approval.
                    ApprovalDecision::Unanswered
                };
                if decision != ApprovalDecision::Unanswered {
                    // Nested on purpose: let-chains are unstable on the
                    // pinned MSRV (1.85) and clippy's collapse lint is
                    // silenced here rather than relaxing it workspace-wide.
                    #[allow(clippy::collapsible_if)]
                    if let Err(error) = transport.refuse_server_request(&request, decision) {
                        self.stopped = Some(StopKind::TransportLost);
                        self.record(RuntimeEventKind::Diagnostic {
                            message: truncate_event_text(&format!(
                                "refusal reply failed; run is terminal: {error}"
                            )),
                        })?;
                        return Err(error);
                    }
                }
                // The harness escalated: it is alive and conversed with, so
                // this interaction is not silence.
                empty_polls = 0;
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!(
                        "refused harness approval request: {:?} ({refusal:?}, {decision:?})",
                        request.method
                    )),
                })?;
            }
            let Some(notification) = notification else {
                continue;
            };
            frames += 1;
            if frames > MAX_TURN_NOTIFICATION_FRAMES {
                self.stopped = Some(StopKind::TransportLost);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text("notification budget exceeded"),
                })?;
                return Err(DriverError::TransportFailed);
            }
            let method = notification
                .get("method")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let params = notification.get("params").cloned().unwrap_or_default();
            // Correlate before absorbing: foreign-thread or foreign-turn
            // frames are this run's observations only if they name this
            // thread. Unidentifiable frames (no ids) are recorded as
            // diagnostics but never stop the run or pollute its usage.
            if !same_turn(&params, thread_id, turn_id) {
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!(
                        "ignored foreign or uncorrelated harness frame: {method}"
                    )),
                })?;
                continue;
            }
            if let Err(error) = self.absorb_notification(&notification) {
                self.stopped = Some(StopKind::TransportLost);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!(
                        "unabsorbable harness frame; run is terminal: {error}"
                    )),
                })?;
                return Err(error);
            }
            if method == "turn/completed" {
                let status = params
                    .pointer("/turn/status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                stop = Some(match status {
                    "completed" => StopKind::Completed,
                    "interrupted" => StopKind::Interrupted,
                    _ => StopKind::Failed,
                });
            }
        }
        self.stopped = stop;
        match self.stopped {
            // Exit(None): the harness reports the interruption; Symbiote
            // never sent CancellationRequested, so CancelAcknowledged would
            // be a tracker-invalid lie.
            Some(StopKind::Completed) => self.record(RuntimeEventKind::Exit { code: Some(0) })?,
            Some(StopKind::Interrupted) => self.record(RuntimeEventKind::Exit { code: None })?,
            _ => self.record(RuntimeEventKind::Exit { code: Some(1) })?,
        }
        Ok(())
    }

    /// Maps one harness notification onto the normalized event stream.
    /// Unknown item types and unknown methods stay Diagnostic (bounded text);
    /// they are observations, not permissions.
    fn absorb_notification(&mut self, notification: &serde_json::Value) -> Result<(), DriverError> {
        let method = notification
            .get("method")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let params = notification.get("params").cloned().unwrap_or_default();
        match method {
            "item/completed" => {
                let item = params.get("item").ok_or(DriverError::MalformedFrame)?;
                let kind = item
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                match kind {
                    "agentMessage" => {
                        let text = item
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .ok_or(DriverError::MalformedFrame)?;
                        self.record(RuntimeEventKind::Message {
                            text: truncate_event_text(text),
                        })?;
                    }
                    "commandExecution" => {
                        let command = item
                            .get("command")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let status = item
                            .get("status")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let output = item
                            .get("aggregatedOutput")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let detail = format!("command [{status}]: {command}\n{output}");
                        self.record(RuntimeEventKind::Observation {
                            resource: ObservationKind::ShellProcess,
                            detail: truncate_event_text(&detail),
                        })?;
                    }
                    "fileChange" => {
                        let status = item
                            .get("status")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let changes = item
                            .get("changes")
                            .map(|c| c.to_string())
                            .unwrap_or_default();
                        let detail = format!("file change [{status}]: {changes}");
                        self.record(RuntimeEventKind::Observation {
                            resource: ObservationKind::FilesystemMutation,
                            detail: truncate_event_text(&detail),
                        })?;
                    }
                    "reasoning" => {
                        // Reasoning summaries are vendor material; retain the
                        // bounded text as a Diagnostic, not as task evidence.
                        let text = item
                            .get("text")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        self.record(RuntimeEventKind::Diagnostic {
                            message: truncate_event_text(&format!("reasoning: {text}")),
                        })?;
                    }
                    _ => {
                        self.record(RuntimeEventKind::Diagnostic {
                            message: truncate_event_text(&format!(
                                "unmapped harness item type: {kind}"
                            )),
                        })?;
                    }
                }
                Ok(())
            }
            "thread/tokenUsage/updated" => {
                let total = params.pointer("/tokenUsage/total");
                let input = total
                    .and_then(|t| t.get("inputTokens"))
                    .and_then(serde_json::Value::as_u64);
                let output = total
                    .and_then(|t| t.get("outputTokens"))
                    .and_then(serde_json::Value::as_u64);
                match (input, output) {
                    (Some(input), Some(output)) => {
                        self.usage_turns += 1;
                        self.record(RuntimeEventKind::Usage {
                            usage: symbiote_runtime_sdk::events::RuntimeUsage {
                                input_tokens:
                                    symbiote_runtime_sdk::events::UsageMeasurement::Measured {
                                        value: input,
                                    },
                                output_tokens:
                                    symbiote_runtime_sdk::events::UsageMeasurement::Measured {
                                        value: output,
                                    },
                                cached_input_tokens:
                                    symbiote_runtime_sdk::events::UsageMeasurement::Unknown {
                                        reason:
                                            symbiote_runtime_sdk::events::UnknownUsageReason::NotReported,
                                    },
                                billed_micro_units:
                                    symbiote_runtime_sdk::events::UsageMeasurement::Unknown {
                                        reason:
                                            symbiote_runtime_sdk::events::UnknownUsageReason::NotReported,
                                    },
                                coverage:
                                    symbiote_runtime_sdk::events::UsageCoverage::AggregateOnly,
                            },
                        })?;
                    }
                    _ => {
                        self.unreported_usage_turns += 1;
                        self.record(RuntimeEventKind::Diagnostic {
                            message: truncate_event_text("usage notification missing fields"),
                        })?;
                    }
                }
                Ok(())
            }
            "error" => {
                let message = params
                    .pointer("/error/message")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!("harness error: {message}")),
                })?;
                Ok(())
            }
            // Turn/item lifecycle frames are consumed by observe_turn, which
            // keys the stop state off them; they need no separate mapping.
            "turn/completed" | "turn/started" | "item/started" => Ok(()),
            _ => {
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!(
                        "unmapped harness notification: {method}"
                    )),
                })?;
                Ok(())
            }
        }
    }

    /// A worker (or test harness) explicitly files its completion report;
    /// the Host treats it as RequestCompletion evidence, never as completion.
    /// Refused once the run reached a terminal stop: the `CompletionRequested`
    /// event must be tracker-replayable (Running), and it can never follow a
    /// terminal `Exit`.
    pub fn request_completion(&mut self, report: &str) -> Result<(), DriverError> {
        if self.completion_report.is_some() {
            return Err(DriverError::AlreadyComplete);
        }
        if self.stopped.is_some() {
            return Err(DriverError::ContractMismatch);
        }
        if report.trim().is_empty() || report.len() > MAX_REPORT_BYTES {
            return Err(DriverError::InvalidInput);
        }
        self.completion_report = Some(report.to_owned());
        self.record(RuntimeEventKind::CompletionRequested {
            report: truncate_event_text(report),
        })?;
        Ok(())
    }
}

/// Thread/turn ids are the driver's correlation keys. They must be bounded
/// and confined to the pinned protocol's identifier charset so exact
/// string equality is meaningful and no control/whitespace aliasing can
/// make a frame look foreign (or foreign frames look ours).
fn valid_correlation_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
}

fn same_turn(params: &serde_json::Value, thread_id: &str, turn_id: &str) -> bool {
    // A frame without an explicit threadId is foreign by default: it cannot
    // be attributed to this run, so it must not drive this run's state.
    let same_thread = params.get("threadId").and_then(serde_json::Value::as_str) == Some(thread_id);
    let turn_field = params.get("turnId").and_then(serde_json::Value::as_str);
    same_thread && turn_field.is_none_or(|id| id == turn_id)
}

/// EventText permits 16 KiB; harness text may legally be larger, so event
/// payloads truncate on a char boundary instead of failing. Truncation is
/// visible (the marker), never silent.
fn truncate_event_text(text: &str) -> EventText {
    const MARKER: &str = "…[truncated]";
    if text.len() <= MAX_EVENT_TEXT_BYTES {
        return EventText::new(text).expect("within the checked bound");
    }
    let budget = MAX_EVENT_TEXT_BYTES - MARKER.len();
    let mut end = budget;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = String::from(&text[..end]);
    truncated.push_str(MARKER);
    EventText::new(truncated).expect("bounded by construction")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, VecDeque};

    /// Deterministic offline fixture server: scripted frames consumed in
    /// order. Calls, notifications and refusals are recorded for assertions.
    /// Not a mock of verification — the driver's real logic runs against it.
    struct FixtureTransport {
        /// Response frames for `call`, keyed by arrival order.
        responses: Vec<Result<serde_json::Value, DriverError>>,
        /// Frames surfaced by `recv_notification` (None = end of script; the
        /// driver treats prolonged silence as a lost transport).
        notifications: VecDeque<Option<serde_json::Value>>,
        /// Server requests awaiting refusal, surfaced by `recv_server_request`.
        server_requests: VecDeque<ServerRequest>,
        refusals: Vec<(String, ApprovalDecision)>,
        /// (kind, method) of everything sent, in order.
        sent: Vec<(&'static str, String)>,
        cursor: usize,
    }
    impl FixtureTransport {
        fn new(
            responses: Vec<Result<serde_json::Value, DriverError>>,
            notifications: Vec<serde_json::Value>,
            server_requests: Vec<ServerRequest>,
        ) -> Self {
            Self {
                responses,
                notifications: notifications
                    .into_iter()
                    .map(Some)
                    .chain(std::iter::once(None))
                    .collect(),
                server_requests: server_requests.into(),
                refusals: Vec::new(),
                sent: Vec::new(),
                cursor: 0,
            }
        }
    }
    impl CodexTransport for FixtureTransport {
        fn call(
            &mut self,
            method: &str,
            _params: &serde_json::Value,
        ) -> Result<serde_json::Value, DriverError> {
            self.sent.push(("request", method.to_owned()));
            let index = self.cursor;
            self.cursor += 1;
            self.responses
                .get(index)
                .cloned()
                .unwrap_or(Err(DriverError::TransportFailed))
        }
        fn notify(&mut self, method: &str, _params: &serde_json::Value) -> Result<(), DriverError> {
            self.sent.push(("notification", method.to_owned()));
            Ok(())
        }
        fn recv_notification(&mut self) -> Result<Option<serde_json::Value>, DriverError> {
            Ok(self.notifications.pop_front().flatten())
        }
        fn recv_server_request(&mut self) -> Result<Option<ServerRequest>, DriverError> {
            Ok(self.server_requests.pop_front())
        }
        fn refuse_server_request(
            &mut self,
            request: &ServerRequest,
            decision: ApprovalDecision,
        ) -> Result<(), DriverError> {
            self.refusals.push((request.method.clone(), decision));
            Ok(())
        }
    }

    fn server_request(
        id: impl Into<serde_json::Value>,
        method: &str,
        params: serde_json::Value,
    ) -> ServerRequest {
        ServerRequest {
            id: id.into(),
            method: method.to_owned(),
            params,
        }
    }

    fn initialize_response() -> serde_json::Value {
        serde_json::json!({"userAgent": format!(
            "symbiote/{} (Linux)",
            symbiote_runtime_discovery::codex::CODEX_VERSION
        )})
    }
    fn thread_start_response() -> serde_json::Value {
        serde_json::json!({"thread": {"id": "thr-1"}})
    }
    fn turn_start_response() -> serde_json::Value {
        serde_json::json!({"turn": {"id": "turn-1"}})
    }
    fn turn_completed(status: &str) -> serde_json::Value {
        serde_json::json!({
            "method": "turn/completed",
            "params": {"threadId": "thr-1", "turnId": "turn-1",
                "turn": {"id": "turn-1", "status": status, "items": []}}
        })
    }

    /// Replays every recorded event through the SDK's session tracker in
    /// order. Any rejection (sequence gap, invalid transition, capacity)
    /// means the driver's journal is not consumable by Host-side tooling.
    /// Catches event-ordering lies empirically instead of trusting the
    /// driver to emit what the SDK accepts.
    fn replay_through_tracker(session: &ExternalSession) {
        let mut tracker = symbiote_runtime_sdk::events::SessionTracker::new(
            session.binding().clone(),
            symbiote_runtime_sdk::events::TrackerLimits::default(),
        )
        .unwrap();
        for event in session.events() {
            tracker
                .apply(event.clone())
                .unwrap_or_else(|error| panic!("tracker rejected event: {error}"));
        }
    }

    fn dispatch(task: &Task) -> Dispatch {
        let host_id = HostId::new("host").unwrap();
        let profile = RuntimeProfile {
            id: RuntimeProfileId::new("profile").unwrap(),
            revision: Revision(0),
            runtime: RuntimeKind::ExternalHarness,
            adapter: AgentRuntimeAdapterId::new("codex-harness").unwrap(),
            installation: Some(InstallationId::new("codex-0-118-0").unwrap()),
            provider: ProviderConnectionId::new("external").unwrap(),
            credential: CredentialReferenceId::new("credential").unwrap(),
            billing_entitlement: BillingEntitlementId::new("ent").unwrap(),
            model: ModelId::new("model-fixture").unwrap(),
            eligible_hosts: BTreeSet::from([host_id.clone()]),
        };
        let binding = WorkforceBinding {
            id: BindingId::new("binding").unwrap(),
            revision: Revision(0),
            project_id: task.project_id().clone(),
            role_id: task.role_id().clone(),
            profile_id: profile.id.clone(),
            profile_revision: profile.revision,
            protocol: VersionedProtocol {
                id: ProtocolId::new("protocol").unwrap(),
                revision: Revision(1),
            },
            access: AccessSnapshot {
                project_id: task.project_id().clone(),
                roots: BTreeSet::from([task.root_id().clone()]),
                grants: BTreeSet::from([Permission::MutateStream]),
                policy_revision: Revision(1),
            },
            required_controls: BTreeSet::new(),
            context: ContextPolicy {
                bundle: ContextBundleId::new("context").unwrap(),
                revision: Revision(1),
                max_input_tokens: 500,
                reserved_output_tokens: 300,
            },
            required_tools: BTreeSet::from(["edit_file".to_string()]),
            required_skills: BTreeSet::new(),
            escalation: EscalationPolicy::StopAndRequestHuman,
        };
        let host = Host {
            id: host_id,
            revision: Revision(0),
            device: DeviceId::new("device").unwrap(),
            fabric: None,
            supported_runtimes: vec![RuntimeKind::ExternalHarness],
            controls: [
                Control::Filesystem,
                Control::Cancellation,
                Control::CompletionAuthority,
            ]
            .into_iter()
            .map(|c| {
                (
                    c,
                    EnforcementClaim {
                        strength: EnforcementStrength::HostEnforced,
                        evidence: EvidenceId::new("proof").unwrap(),
                        verified_at: Timestamp(1),
                        expires_at: Timestamp(1_000_000),
                    },
                )
            })
            .collect(),
        };
        let role = Role {
            id: task.role_id().clone(),
            project_id: task.project_id().clone(),
            revision: Revision(0),
            name: "Engineer".into(),
            operating_contract: VersionedRoleContract {
                id: RoleContractId::new("contract").unwrap(),
                revision: Revision(1),
            },
        };
        Dispatch::compile(
            DispatchId::new("dispatch").unwrap(),
            RuntimeContractId::new("rtc").unwrap(),
            symbiote_domain::DispatchInputs {
                task,
                role: &role,
                binding: &binding,
                profile: &profile,
                host: &host,
                minimum_enforcement: &std::collections::BTreeMap::new(),
                now: Timestamp(10),
            },
        )
        .unwrap()
    }

    fn task() -> Task {
        Task::new(
            TaskId::new("task").unwrap(),
            ProjectId::new("project").unwrap(),
            RootId::new("root").unwrap(),
            RoleId::new("role").unwrap(),
            ChangeStreamId::new("stream").unwrap(),
            VersionedTaskContract {
                id: TaskContractId::new("contract").unwrap(),
                revision: Revision(1),
            },
        )
    }

    fn session() -> ExternalSession {
        ExternalSession::new(&dispatch(&task()), Timestamp(20)).unwrap()
    }

    fn transport_with(notifications: Vec<serde_json::Value>) -> FixtureTransport {
        FixtureTransport::new(
            vec![
                Ok(initialize_response()),
                Ok(thread_start_response()),
                Ok(turn_start_response()),
            ],
            notifications,
            vec![],
        )
    }

    #[test]
    fn single_turn_completes_with_message_and_usage() {
        let mut session = session();
        let notifications = vec![
            serde_json::json!({
                "method": "item/completed",
                "params": {"threadId": "thr-1", "turnId": "turn-1",
                    "item": {"type": "agentMessage", "id": "i1", "text": "done"}}
            }),
            serde_json::json!({
                "method": "thread/tokenUsage/updated",
                "params": {"threadId": "thr-1", "turnId": "turn-1",
                    "tokenUsage": {"last": {}, "total": {
                        "inputTokens": 120, "outputTokens": 40,
                        "cachedInputTokens": 0, "reasoningOutputTokens": 0, "totalTokens": 160}}}
            }),
            turn_completed("completed"),
        ];
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        assert_eq!(session.harness_thread_id(), Some("thr-1"));
        let kinds: Vec<_> = session.events().iter().map(|e| e.payload()).collect();
        assert!(matches!(kinds[0], RuntimeEventKind::Ready {}));
        assert!(matches!(kinds[1], RuntimeEventKind::Message { .. }));
        assert!(matches!(kinds[2], RuntimeEventKind::Usage { .. }));
        assert!(matches!(kinds[3], RuntimeEventKind::Exit { code: Some(0) }));
        let run = session.run_summary();
        assert_eq!(run.turns_observed, 1);
        assert_eq!(run.usage_turns, 1);
        assert_eq!(run.unreported_usage_turns, 0);
        assert_eq!(run.stopped, Some(StopKind::Completed));
        assert_eq!(run.dispatch_id, dispatch(&task()).id().clone());
        // All events carry the external harness binding.
        assert!(
            session
                .events()
                .iter()
                .all(|e| e.binding().runtime == RuntimeKind::ExternalHarness)
        );
        // The journal is contiguously sequenced, uniquely identified and
        // fully replayable through the SDK's session tracker.
        assert!(
            session
                .events()
                .iter()
                .enumerate()
                .all(|(i, e)| e.sequence() as usize == i + 1)
        );
        let ids: BTreeSet<_> = session.events().iter().map(|e| e.id().clone()).collect();
        assert_eq!(ids.len(), session.events().len());
        replay_through_tracker(&session);
    }

    #[test]
    fn terminal_stop_blocks_later_completion_and_turns() {
        let mut session = session();
        let mut transport = transport_with(vec![turn_completed("completed")]);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        // After a terminal stop, no further turn can start and no completion
        // report can be filed: the CompletionRequested event would follow a
        // terminal Exit and be unreplayable.
        assert_eq!(
            session.turn("another turn", &mut transport),
            Err(DriverError::AlreadyComplete)
        );
        assert_eq!(
            session.start_turn("another turn", "/tmp/worktree", &mut transport),
            Err(DriverError::AlreadyComplete)
        );
        assert_eq!(
            session.request_completion("done"),
            Err(DriverError::ContractMismatch)
        );
        // Only one Ready was ever recorded; nothing was appended.
        assert_eq!(
            session
                .events()
                .iter()
                .filter(|e| matches!(e.payload(), RuntimeEventKind::Ready {}))
                .count(),
            1
        );
        assert!(matches!(
            session.events().last().unwrap().payload(),
            RuntimeEventKind::Exit { .. }
        ));
        replay_through_tracker(&session);
    }

    #[test]
    fn unattributable_notifications_never_stop_or_pollute_the_run() {
        let mut session = session();
        let notifications = vec![
            // No threadId: unattributable, must be ignored, not absorbed.
            serde_json::json!({
                "method": "thread/tokenUsage/updated",
                "params": {"tokenUsage": {"last": {}, "total": {
                    "inputTokens": 9999, "outputTokens": 9999,
                    "cachedInputTokens": 0, "reasoningOutputTokens": 0, "totalTokens": 19998}}}
            }),
            // No ids at all: unattributable, must be ignored.
            serde_json::json!({"method": "thread/status/changed", "params": {}}),
            turn_completed("completed"),
        ];
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        let run = session.run_summary();
        assert_eq!(run.stopped, Some(StopKind::Completed));
        // The foreign-frame usage never touched this run's accounting.
        assert_eq!(run.usage_turns, 0);
        assert_eq!(run.unreported_usage_turns, 0);
        // Both foreign frames left bounded, distinct diagnostics.
        let ignored: usize = session
            .events()
            .iter()
            .filter(|e| {
                matches!(
                    e.payload(),
                    RuntimeEventKind::Diagnostic { message }
                        if message
                            .as_str()
                            .contains("foreign or uncorrelated harness frame")
                )
            })
            .count();
        assert_eq!(ignored, 2);
        replay_through_tracker(&session);
    }

    #[test]
    fn failed_handshake_is_terminal_and_retry_never_doubles_ready() {
        // A server reporting a foreign version is refused after Ready; the
        // session must be terminal so a retry cannot append a second Ready
        // (which the SDK tracker would reject) to a half-open journal.
        for bad_agent in ["symbiote/0.117.0", "unrelated", "symbiote/0.118.0evil"] {
            let mut session = session();
            let mut transport = FixtureTransport::new(
                vec![Ok(serde_json::json!({
                    "userAgent": format!("{bad_agent} (Linux)")
                }))],
                vec![],
                vec![],
            );
            let error = session
                .begin_thread("/tmp/worktree", &mut transport)
                .unwrap_err();
            assert_eq!(error, DriverError::UnsupportedVersion);
            // Terminal: no retry, no turn, no completion filing.
            assert_eq!(
                session.begin_thread("/tmp/worktree", &mut transport),
                Err(DriverError::AlreadyComplete)
            );
            assert_eq!(
                session.turn("another", &mut transport),
                Err(DriverError::AlreadyComplete)
            );
            assert_eq!(
                session.request_completion("done"),
                Err(DriverError::ContractMismatch)
            );
            // Exactly one Ready, one refusal diagnostic, tracker-replayable.
            assert_eq!(
                session
                    .events()
                    .iter()
                    .filter(|e| matches!(e.payload(), RuntimeEventKind::Ready {}))
                    .count(),
                1
            );
            assert!(matches!(
                session.run_summary().stopped,
                Some(StopKind::TransportLost)
            ));
            replay_through_tracker(&session);
        }
    }

    #[test]
    fn malformed_thread_response_is_terminal_too() {
        let mut session = session();
        let mut transport = FixtureTransport::new(
            vec![Ok(initialize_response()), Ok(serde_json::json!({}))],
            vec![],
            vec![],
        );
        let error = session
            .begin_thread("/tmp/worktree", &mut transport)
            .unwrap_err();
        assert_eq!(error, DriverError::MalformedFrame);
        assert_eq!(
            session.begin_thread("/tmp/worktree", &mut transport),
            Err(DriverError::AlreadyComplete)
        );
        replay_through_tracker(&session);
    }

    #[test]
    fn malformed_or_charset_violating_harness_ids_are_refused() {
        for bad_thread in [
            serde_json::json!({"thread": {"id": ""}}),
            serde_json::json!({"thread": {"id": "thr 1"}}),
            serde_json::json!({"thread": {"id": "thr\t1"}}),
            serde_json::json!({"thread": {"id": "thré"}}),
            serde_json::json!({"thread": {"id": "x".repeat(129)}}),
            serde_json::json!({"thread": {"id": 17}}),
        ] {
            let mut session = session();
            let mut transport = FixtureTransport::new(vec![Ok(bad_thread)], vec![], vec![]);
            assert!(matches!(
                session.start_turn("fix the bug", "/tmp/worktree", &mut transport),
                Err(DriverError::MalformedFrame)
            ));
        }
    }

    #[test]
    fn sequence_numbers_remain_contiguous_for_long_runs() {
        // Long enough that the event id's sequence number grows a digit and
        // the suffix slice shrinks accordingly.
        let mut session = session();
        let notifications: Vec<serde_json::Value> = (0..150)
            .map(|i| {
                serde_json::json!({
                    "method": "item/completed",
                    "params": {"threadId": "thr-1", "turnId": "turn-1",
                        "item": {"type": "agentMessage", "id": format!("i{i}"),
                            "text": format!("progress {i}")}}
                })
            })
            .chain(std::iter::once(turn_completed("completed")))
            .collect();
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        assert!(
            session
                .events()
                .iter()
                .enumerate()
                .all(|(i, e)| e.sequence() as usize == i + 1)
        );
        let ids: BTreeSet<_> = session.events().iter().map(|e| e.id().as_str()).collect();
        assert_eq!(ids.len(), session.events().len());
        replay_through_tracker(&session);
    }

    #[test]
    fn failed_turn_yields_exit_one_and_failed_stop() {
        let mut session = session();
        let mut transport = transport_with(vec![turn_completed("failed")]);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        let run = session.run_summary();
        assert_eq!(run.stopped, Some(StopKind::Failed));
        assert!(matches!(
            session.events().last().unwrap().payload(),
            RuntimeEventKind::Exit { code: Some(1) }
        ));
    }

    #[test]
    fn command_and_file_observations_are_recorded_as_observations() {
        let mut session = session();
        let notifications = vec![
            serde_json::json!({
                "method": "item/completed",
                "params": {"threadId": "thr-1", "turnId": "turn-1",
                    "item": {"type": "commandExecution", "id": "i1",
                        "command": "cargo test", "status": "completed",
                        "aggregatedOutput": "ok", "exitCode": 0}}
            }),
            serde_json::json!({
                "method": "item/completed",
                "params": {"threadId": "thr-1", "turnId": "turn-1",
                    "item": {"type": "fileChange", "id": "i2", "status": "completed",
                        "changes": [{"path": "src/main.rs", "kind": "update"}]}}
            }),
            turn_completed("completed"),
        ];
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        let observations: Vec<_> = session
            .events()
            .iter()
            .filter_map(|e| match e.payload() {
                RuntimeEventKind::Observation { resource, .. } => Some(resource.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            observations,
            vec![
                ObservationKind::ShellProcess,
                ObservationKind::FilesystemMutation
            ]
        );
    }

    #[test]
    fn every_approval_request_is_refused_and_counted() {
        let mut session = session();
        let mut transport = FixtureTransport::new(
            vec![
                Ok(initialize_response()),
                Ok(thread_start_response()),
                Ok(turn_start_response()),
            ],
            vec![turn_completed("completed")],
            vec![
                server_request(
                    1,
                    "execCommandApproval",
                    serde_json::json!({"callId": "c1", "conversationId": "thr-1",
                        "command": ["rm", "-rf"], "cwd": "/tmp/worktree", "parsedCmd": []}),
                ),
                server_request(
                    2,
                    "applyPatchApproval",
                    serde_json::json!({"callId": "c2", "conversationId": "thr-1",
                        "fileChanges": {}}),
                ),
                server_request(
                    "srv-3",
                    "item/permissions/requestApproval",
                    serde_json::json!({"itemId": "i1", "threadId": "thr-1",
                        "turnId": "turn-1", "permissions": {}}),
                ),
                server_request(
                    "srv-4",
                    "mcpServer/elicitation/request",
                    serde_json::json!({"serverName": "srv", "threadId": "thr-1"}),
                ),
                server_request(
                    "srv-5",
                    "item/tool/requestUserInput",
                    serde_json::json!({"itemId": "i2", "threadId": "thr-1",
                        "turnId": "turn-1", "questions": []}),
                ),
            ],
        );
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        // Four known shapes get a pinned denial reply; the user-input
        // question gets none (no invented answers).
        assert_eq!(transport.refusals.len(), 4);
        assert!(
            transport
                .refusals
                .iter()
                .all(|(_, decision)| *decision == ApprovalDecision::Denied)
        );
        assert!(
            !transport
                .refusals
                .iter()
                .any(|(method, _)| method == "item/tool/requestUserInput")
        );
        let run = session.run_summary();
        assert_eq!(run.approvals_refused, 5);
        let refusal_diagnostics: Vec<_> = session
            .events()
            .iter()
            .filter_map(|e| match e.payload() {
                RuntimeEventKind::Diagnostic { message }
                    if message
                        .as_str()
                        .contains("refused harness approval request") =>
                {
                    Some(message.as_str().to_owned())
                }
                _ => None,
            })
            .collect();
        assert_eq!(refusal_diagnostics.len(), 5);
        // The refusal diagnostics never leak the harness's request contents
        // (the fixture's exec request carries "rm -rf"; only the method name
        // may appear, and no method contains it).
        assert!(!refusal_diagnostics.iter().any(|m| m.contains("\"rm\"")));
        // Every refusal names the refused method, not its payload.
        assert!(
            refusal_diagnostics
                .iter()
                .any(|m| m.contains("execCommandApproval"))
        );
    }

    #[test]
    fn unknown_server_request_is_counted_but_never_answered() {
        let mut session = session();
        let mut transport = FixtureTransport::new(
            vec![
                Ok(initialize_response()),
                Ok(thread_start_response()),
                Ok(turn_start_response()),
            ],
            vec![turn_completed("completed")],
            vec![server_request(
                "srv-7",
                "account/chatgptAuthTokens/refresh",
                serde_json::json!({"reason": "test"}),
            )],
        );
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        // Unknown methods get no reply: no refusal body is invented for a
        // shape outside the pinned approval responses.
        assert!(transport.refusals.is_empty());
        assert_eq!(session.run_summary().approvals_refused, 1);
        let refusals: Vec<_> = session
            .events()
            .iter()
            .filter_map(|e| match e.payload() {
                RuntimeEventKind::Diagnostic { message }
                    if message
                        .as_str()
                        .contains("refused harness approval request") =>
                {
                    Some(message.as_str().to_owned())
                }
                _ => None,
            })
            .collect();
        assert_eq!(refusals.len(), 1);
        assert!(refusals[0].contains("Unanswered"));
    }

    #[test]
    fn transport_loss_during_turn_is_explicit_not_completelike() {
        let mut session = session();
        let mut transport = transport_with(vec![]);
        let error = session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap_err();
        assert_eq!(error, DriverError::TransportFailed);
        let run = session.run_summary();
        assert_eq!(run.stopped, Some(StopKind::TransportLost));
        // A lost transport never reads as a normal stop.
        assert_ne!(run.stopped, Some(StopKind::Completed));
        // The silence budget fired, not a fabricated completion.
        assert!(session.events().iter().any(
            |e| matches!(e.payload(), RuntimeEventKind::Diagnostic { message }
                    if message.as_str().contains("harness silent beyond the poll budget"))
        ));
        assert!(
            !session
                .events()
                .iter()
                .any(|e| matches!(e.payload(), RuntimeEventKind::Exit { .. }))
        );
    }

    #[test]
    fn driver_and_transport_work_end_to_end_over_a_real_subprocess() {
        // The full driver against the real framed transport on a real
        // process (pipes, EOF, deadlines) — scripted frames, no codex
        // binary, no network, no credentials.
        let mut session = session();
        let mut transport = process::spawn_test_server(
            "printf '%s\\n' \
             '{\"id\":1,\"result\":{\"userAgent\":\"symbiote/0.118.0 (Linux)\"}}' \
             '{\"id\":2,\"result\":{\"thread\":{\"id\":\"thr-1\"}}}' \
             '{\"id\":3,\"result\":{\"turn\":{\"id\":\"turn-1\"}}}' \
             '{\"method\":\"item/completed\",\"params\":{\"threadId\":\"thr-1\",\"turnId\":\"turn-1\",\"item\":{\"type\":\"agentMessage\",\"id\":\"i1\",\"text\":\"done\"}}}' \
             '{\"method\":\"thread/tokenUsage/updated\",\"params\":{\"threadId\":\"thr-1\",\"turnId\":\"turn-1\",\"tokenUsage\":{\"last\":{},\"total\":{\"inputTokens\":11,\"outputTokens\":5,\"cachedInputTokens\":0,\"reasoningOutputTokens\":0,\"totalTokens\":16}}}}' \
             '{\"method\":\"turn/completed\",\"params\":{\"threadId\":\"thr-1\",\"turnId\":\"turn-1\",\"turn\":{\"id\":\"turn-1\",\"status\":\"completed\",\"items\":[]}}}'; sleep 30",
        );
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        let kinds: Vec<_> = session.events().iter().map(|e| e.payload()).collect();
        assert!(matches!(kinds[0], RuntimeEventKind::Ready {}));
        assert!(matches!(kinds[1], RuntimeEventKind::Message { .. }));
        assert!(matches!(kinds[2], RuntimeEventKind::Usage { .. }));
        assert!(matches!(kinds[3], RuntimeEventKind::Exit { code: Some(0) }));
        let run = session.run_summary();
        assert_eq!(run.stopped, Some(StopKind::Completed));
        assert_eq!(run.usage_turns, 1);
        replay_through_tracker(&session);
    }

    #[test]
    fn silent_harness_never_reads_as_completion_or_hangs() {
        // A harness that starts a turn and then goes quiet: the driver's
        // poll budget must convert sustained silence into an explicit lost
        // transport instead of spinning forever or inventing an outcome.
        let mut session = session();
        let mut transport = process::spawn_test_server(
            "printf '%s\\n' \
             '{\"id\":1,\"result\":{\"userAgent\":\"symbiote/0.118.0 (Linux)\"}}' \
             '{\"id\":2,\"result\":{\"thread\":{\"id\":\"thr-1\"}}}' \
             '{\"id\":3,\"result\":{\"turn\":{\"id\":\"turn-1\"}}}'; sleep 1",
        );
        let error = session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap_err();
        assert_eq!(error, DriverError::TransportFailed);
        assert_eq!(session.run_summary().stopped, Some(StopKind::TransportLost));
        // The run recorded a transport diagnostic and never an Exit: silence
        // produces no fabricated completion either way.
        assert!(
            session
                .events()
                .iter()
                .any(|e| matches!(e.payload(), RuntimeEventKind::Diagnostic { .. }))
        );
        assert!(
            !session
                .events()
                .iter()
                .any(|e| matches!(e.payload(), RuntimeEventKind::Exit { .. }))
        );
        replay_through_tracker(&session);
    }

    #[test]
    fn notification_flood_is_bounded() {
        let mut session = session();
        let flood: Vec<serde_json::Value> = (0..MAX_TURN_NOTIFICATION_FRAMES + 1)
            .map(|_| {
                serde_json::json!({"method": "thread/status/changed",
                    "params": {"threadId": "thr-1", "status": "idle"}})
            })
            .collect();
        let mut transport = transport_with(flood);
        let error = session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap_err();
        assert_eq!(error, DriverError::TransportFailed);
        // Event count stays bounded by the flood budget, not unbounded.
        assert!(session.events().len() <= MAX_TURN_NOTIFICATION_FRAMES + 8);
    }

    #[test]
    fn native_dispatch_is_refused_by_the_external_driver() {
        let host_id = HostId::new("host").unwrap();
        let native_task = task();
        let profile = RuntimeProfile {
            id: RuntimeProfileId::new("profile").unwrap(),
            revision: Revision(0),
            runtime: RuntimeKind::NativeSymbiote,
            adapter: AgentRuntimeAdapterId::new("native-agent").unwrap(),
            installation: None,
            provider: ProviderConnectionId::new("native").unwrap(),
            credential: CredentialReferenceId::new("credential").unwrap(),
            billing_entitlement: BillingEntitlementId::new("ent").unwrap(),
            model: ModelId::new("model-fixture").unwrap(),
            eligible_hosts: BTreeSet::from([host_id.clone()]),
        };
        let binding = WorkforceBinding {
            id: BindingId::new("binding").unwrap(),
            revision: Revision(0),
            project_id: native_task.project_id().clone(),
            role_id: native_task.role_id().clone(),
            profile_id: profile.id.clone(),
            profile_revision: profile.revision,
            protocol: VersionedProtocol {
                id: ProtocolId::new("protocol").unwrap(),
                revision: Revision(1),
            },
            access: AccessSnapshot {
                project_id: native_task.project_id().clone(),
                roots: BTreeSet::from([native_task.root_id().clone()]),
                grants: BTreeSet::from([Permission::MutateStream]),
                policy_revision: Revision(1),
            },
            required_controls: BTreeSet::new(),
            context: ContextPolicy {
                bundle: ContextBundleId::new("context").unwrap(),
                revision: Revision(1),
                max_input_tokens: 500,
                reserved_output_tokens: 300,
            },
            required_tools: BTreeSet::new(),
            required_skills: BTreeSet::new(),
            escalation: EscalationPolicy::StopAndRequestHuman,
        };
        let host = Host {
            id: host_id,
            revision: Revision(0),
            device: DeviceId::new("device").unwrap(),
            fabric: None,
            supported_runtimes: vec![RuntimeKind::NativeSymbiote],
            controls: [
                Control::Filesystem,
                Control::Cancellation,
                Control::CompletionAuthority,
            ]
            .into_iter()
            .map(|c| {
                (
                    c,
                    EnforcementClaim {
                        strength: EnforcementStrength::HostEnforced,
                        evidence: EvidenceId::new("proof").unwrap(),
                        verified_at: Timestamp(1),
                        expires_at: Timestamp(1_000_000),
                    },
                )
            })
            .collect(),
        };
        let role = Role {
            id: native_task.role_id().clone(),
            project_id: native_task.project_id().clone(),
            revision: Revision(0),
            name: "Engineer".into(),
            operating_contract: VersionedRoleContract {
                id: RoleContractId::new("contract").unwrap(),
                revision: Revision(1),
            },
        };
        let native_dispatch = Dispatch::compile(
            DispatchId::new("dispatch").unwrap(),
            RuntimeContractId::new("rtc").unwrap(),
            symbiote_domain::DispatchInputs {
                task: &native_task,
                role: &role,
                binding: &binding,
                profile: &profile,
                host: &host,
                minimum_enforcement: &std::collections::BTreeMap::new(),
                now: Timestamp(10),
            },
        )
        .unwrap();
        assert!(matches!(
            ExternalSession::new(&native_dispatch, Timestamp(20)),
            Err(DriverError::NotExternalHarness)
        ));
    }

    #[test]
    fn expired_contract_and_bad_inputs_are_refused() {
        assert!(matches!(
            ExternalSession::new(&dispatch(&task()), Timestamp(2_000_000)),
            Err(DriverError::InvalidContract)
        ));
        let mut session = session();
        let mut transport = transport_with(vec![]);
        assert!(matches!(
            session.start_turn("  ", "/tmp/worktree", &mut transport),
            Err(DriverError::InvalidInput)
        ));
        assert!(matches!(
            session.start_turn("fix the bug", "relative/path", &mut transport),
            Err(DriverError::InvalidInput)
        ));
        assert!(matches!(
            session.request_completion(""),
            Err(DriverError::InvalidInput)
        ));
        assert!(matches!(
            session.request_completion(&"x".repeat(MAX_REPORT_BYTES + 1)),
            Err(DriverError::InvalidInput)
        ));
    }

    #[test]
    fn completion_report_is_evidence_and_double_filing_is_refused() {
        let mut session = session();
        session.request_completion("implemented the slice").unwrap();
        assert_eq!(
            session.request_completion("again"),
            Err(DriverError::AlreadyComplete)
        );
        let kinds: Vec<_> = session.events().iter().map(|e| e.payload()).collect();
        assert!(matches!(
            kinds[0],
            RuntimeEventKind::CompletionRequested { .. }
        ));
        let run = session.run_summary();
        assert_eq!(
            run.completion_report.as_deref(),
            Some("implemented the slice")
        );
        // The report is evidence routed to the Host; it is not a state change.
        assert!(run.stopped.is_none());
    }

    #[test]
    fn oversized_harness_text_truncates_on_char_boundaries() {
        let mut session = session();
        let big = "🔍".repeat(MAX_EVENT_TEXT_BYTES); // 4-byte chars
        let notifications = vec![
            serde_json::json!({
                "method": "item/completed",
                "params": {"threadId": "thr-1", "turnId": "turn-1",
                    "item": {"type": "agentMessage", "id": "i1", "text": big}}
            }),
            turn_completed("completed"),
        ];
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        let message = session
            .events()
            .iter()
            .find_map(|e| match e.payload() {
                RuntimeEventKind::Message { text } => Some(text.as_str().to_owned()),
                _ => None,
            })
            .unwrap();
        assert!(message.len() <= MAX_EVENT_TEXT_BYTES);
        assert!(message.ends_with("…[truncated]"));
        assert!(message.chars().all(|c| c != char::REPLACEMENT_CHARACTER));
    }

    #[test]
    fn usage_missing_fields_counts_as_unreported_not_free() {
        let mut session = session();
        let notifications = vec![
            serde_json::json!({
                "method": "thread/tokenUsage/updated",
                "params": {"threadId": "thr-1", "turnId": "turn-1",
                    "tokenUsage": {"last": {}, "total": {
                        "inputTokens": 10, "outputTokens": null,
                        "cachedInputTokens": 0, "reasoningOutputTokens": 0, "totalTokens": 10}}}
            }),
            turn_completed("completed"),
        ];
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        let run = session.run_summary();
        assert_eq!(run.usage_turns, 0);
        assert_eq!(run.unreported_usage_turns, 1);
    }

    #[test]
    fn interrupted_turn_records_exit_none_and_stays_tracker_replayable() {
        let mut session = session();
        let mut transport = transport_with(vec![turn_completed("interrupted")]);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        assert_eq!(session.run_summary().stopped, Some(StopKind::Interrupted));
        // Exit(None), not CancelAcknowledged: the driver never sent
        // CancellationRequested, so an acknowledgement would be a lie the
        // SDK session tracker rejects (InvalidTransition).
        assert!(matches!(
            session.events().last().unwrap().payload(),
            RuntimeEventKind::Exit { code: None }
        ));
        replay_through_tracker(&session);
    }

    #[test]
    fn foreign_turn_notifications_do_not_stop_observation() {
        let mut session = session();
        let notifications = vec![
            serde_json::json!({
                "method": "turn/completed",
                "params": {"threadId": "thr-1", "turnId": "turn-OTHER",
                    "turn": {"id": "turn-OTHER", "status": "failed", "items": []}}
            }),
            turn_completed("completed"),
        ];
        let mut transport = transport_with(notifications);
        session
            .start_turn("fix the bug", "/tmp/worktree", &mut transport)
            .unwrap();
        assert_eq!(session.run_summary().stopped, Some(StopKind::Completed));
    }
}
