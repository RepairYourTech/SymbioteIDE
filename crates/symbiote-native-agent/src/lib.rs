//! Native Symbiote Agent worker loop: the model/tool turn engine driven by a
//! started dispatch against its resolved provider binding. This crate owns
//! #465's native side. It is deliberately offline-testable: the
//! [`InferenceTransport`] trait abstracts the OpenAI Responses API call so the
//! loop, its budget arithmetic, its event journaling and its completion
//! handoff are exercised without network access or spending. The concrete
//! HTTPS transport lives behind the same trait and requires live credentials
//! and billing authorization that only the user can grant; no silent native
//! billing fallback exists for external execution and vice versa.
//!
//! Boundary: the loop never writes canonical state. It produces typed
//! [`RuntimeEvent`]s (via the SDK's session/event contracts) and a
//! completion report that the Host routes through RequestCompletion →
//! BeginVerification → Complete with independent review. Worker completion is
//! evidence, never authority.
use symbiote_domain::*;
pub mod tools;

/// Forwards through mutable references so a session can hold a boxed
/// executor and still drive the generic execution helper.
impl<T: tools::ShellToolExecutor + ?Sized> tools::ShellToolExecutor for &mut T {
    fn run_shell(
        &mut self,
        invocation: &tools::ShellInvocation,
        worktree: &std::path::Path,
    ) -> Result<(Vec<u8>, Option<i32>), tools::ToolExecError> {
        (**self).run_shell(invocation, worktree)
    }
}
use symbiote_runtime_sdk::events::{EventText, RuntimeEvent, RuntimeEventKind, SessionBinding};
use symbiote_runtime_sdk::provider::{
    FinishReason, InputMessage, InputPart, MessageRole, ModelDescriptor, ProviderError,
    ProviderRequest, ProviderResponse, TokenUsage,
};

pub const NATIVE_LOOP_VERSION: u32 = 1;
pub const MAX_TURNS_PER_RUN: u32 = 64;
/// Hard halt on accumulated assistant text even when usage goes unreported:
/// the backstop that bounds runaway loops a provider reports nothing about.
pub const MAX_ACCUMULATED_OUTPUT_BYTES: usize = 512 * 1024;
/// Bound on total tool-result text entered into the conversation: tool
/// results are growth the provider never reports usage for.
pub const MAX_TOOL_RESULT_BYTES: usize = 256 * 1024;
/// EventText's own limit; reports and transport text are truncated to it
/// rather than panicking.
pub const MAX_EVENT_TEXT_BYTES: usize = 16_384;
/// Completion reports are bounded by the same limit as event text.
pub const MAX_REPORT_BYTES: usize = MAX_EVENT_TEXT_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopError {
    /// The dispatch contract no longer validates (expired enforcement, etc.).
    InvalidContract,
    /// Budget exhausted: input+output tokens exceed the binding context.
    BudgetExhausted,
    /// Turn cap reached without completion or length exhaustion.
    TurnCapReached,
    /// Provider failed terminally; retry policy is the Host's, not the loop's.
    ProviderFailed,
    /// The transport produced an envelope failing SDK validation.
    EnvelopeMismatch,
    /// The model requested a tool the session has no definition for.
    UnknownTool,
    /// Completion was already requested; the loop is finished.
    AlreadyComplete,
    /// Caller input failed local bounds.
    InvalidInput,
}

impl std::fmt::Display for LoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native loop: {self:?}")
    }
}
impl std::error::Error for LoopError {}

/// The inference call boundary, mirroring the SDK's InferenceProviderAdapter
/// separation: implementations translate a validated [`ProviderRequest`] into
/// a [`ProviderResponse`] envelope. The production implementation is the
/// OpenAI Responses API client; tests provide deterministic offline responses.
pub trait InferenceTransport {
    fn request(
        &mut self,
        request: &ProviderRequest,
        model: &ModelDescriptor,
    ) -> Result<ProviderResponse, ProviderError>;
}

/// Lets a Host hold `Box<dyn InferenceTransport>` factories and pass
/// `&mut dyn InferenceTransport` into the generic loop.
impl<T: InferenceTransport + ?Sized> InferenceTransport for &mut T {
    fn request(
        &mut self,
        request: &ProviderRequest,
        model: &ModelDescriptor,
    ) -> Result<ProviderResponse, ProviderError> {
        (**self).request(request, model)
    }
}

/// Deterministic offline transport for tests: scripted responses consumed in
/// order. Not a mock of verification — the loop's real logic runs against it.
pub struct ScriptedTransport {
    responses: Vec<Result<ProviderResponse, ProviderError>>,
    cursor: usize,
    seen_requests: Vec<ProviderRequest>,
}

impl ScriptedTransport {
    pub fn new(responses: Vec<Result<ProviderResponse, ProviderError>>) -> Self {
        Self {
            responses,
            cursor: 0,
            seen_requests: Vec::new(),
        }
    }

    pub fn seen_requests(&self) -> &[ProviderRequest] {
        &self.seen_requests
    }
}

impl InferenceTransport for ScriptedTransport {
    fn request(
        &mut self,
        request: &ProviderRequest,
        _model: &ModelDescriptor,
    ) -> Result<ProviderResponse, ProviderError> {
        self.seen_requests.push(request.clone());
        let index = self.cursor;
        self.cursor += 1;
        self.responses
            .get(index)
            .cloned()
            .unwrap_or(Ok(finish_response(request, FinishReason::Stop)))
    }
}

/// The fallback envelope satisfies the SDK's own response validation
/// (Stop requires non-empty text) and reports known usage so accounting
/// stays exercised even in the scripted path.
fn finish_response(request: &ProviderRequest, finish: FinishReason) -> ProviderResponse {
    let text = if matches!(finish, FinishReason::Stop) {
        "done".to_owned()
    } else {
        String::new()
    };
    ProviderResponse {
        schema_version: request.schema_version,
        request_id: request.request_id.clone(),
        provider_id: request.provider_id.clone(),
        model_id: request.model_id.clone(),
        text,
        tool_calls: Vec::new(),
        finish_reason: finish,
        usage: TokenUsage::Known {
            input_tokens: 1,
            output_tokens: 1,
            cached_input_tokens: None,
            reasoning_tokens: None,
        },
    }
}

/// Durable record of one loop run: the inputs and the emitted events. The
/// Host journals this; the loop itself holds no canonical write authority.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct LoopRun {
    pub version: u32,
    pub dispatch_id: DispatchId,
    pub task_id: TaskId,
    pub provider_id: ProviderConnectionId,
    pub model_id: ModelId,
    pub turns: u32,
    pub input_tokens_used: u64,
    pub output_tokens_used: u64,
    pub unreported_usage_turns: u32,
    pub completion_report: Option<String>,
    pub halted: Option<HaltReason>,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HaltReason {
    Stop,
    Length,
    BudgetExhausted,
    TurnCapReached,
    ProviderFailed,
    EnvelopeMismatch,
    /// The model proposed a tool absent from the dispatch contract.
    UnknownTool,
    /// The accumulated-output byte backstop fired (unreported usage).
    OutputBackstop,
}

/// The native turn engine. Created per dispatch from already-authorized
/// state: the dispatch contract supplies the context budget, tools and
/// identities; the model descriptor supplies the envelope bounds. All
/// emitted events carry the session binding so downstream consumers can
/// attribute every effect.
pub struct NativeSession {
    binding: SessionBinding,
    task_id: TaskId,
    provider_id: ProviderConnectionId,
    tool_result_bytes: usize,
    contract_context: ContextPolicy,
    tools: Vec<String>,
    tool_executor: Option<Box<dyn tools::ShellToolExecutor>>,
    worktree: Option<std::path::PathBuf>,
    model: ModelDescriptor,
    messages: Vec<InputMessage>,
    events: Vec<RuntimeEvent>,
    next_event_sequence: u64,
    turns: u32,
    input_tokens: u64,
    output_tokens: u64,
    unreported_usage_turns: u32,
    accumulated_output_bytes: usize,
    completion_report: Option<String>,
    halted: Option<HaltReason>,
    request_counter: u64,
}

impl NativeSession {
    /// The dispatch must still validate at `now` (enforcement windows, host
    /// eligibility) — this is re-checked, not trusted from preparation time.
    pub fn new(
        dispatch: &Dispatch,
        model: ModelDescriptor,
        at: Timestamp,
    ) -> Result<Self, LoopError> {
        dispatch
            .contract()
            .validate_at(at)
            .map_err(|_| LoopError::InvalidContract)?;
        let contract = dispatch.contract();
        let session = SessionId::new(format!("sess_{}", dispatch.id().as_str()))
            .map_err(|_| LoopError::InvalidContract)?;
        Ok(Self {
            binding: SessionBinding {
                session_id: session,
                dispatch_id: dispatch.id().clone(),
                host_id: contract.host_id().clone(),
                runtime: RuntimeKind::NativeSymbiote,
            },
            task_id: contract.task_id().clone(),
            provider_id: contract.profile().provider.clone(),
            contract_context: contract.binding().context.clone(),
            tools: contract.binding().required_tools.iter().cloned().collect(),
            tool_executor: None,
            tool_result_bytes: 0,
            worktree: None,
            model,
            messages: Vec::new(),
            events: Vec::new(),
            next_event_sequence: 1,
            turns: 0,
            input_tokens: 0,
            output_tokens: 0,
            unreported_usage_turns: 0,
            accumulated_output_bytes: 0,
            completion_report: None,
            halted: None,
            request_counter: 0,
        })
    }

    pub fn binding(&self) -> &SessionBinding {
        &self.binding
    }

    /// Enables host-gated tool execution: declared shell tools run through
    /// the injected executor inside the given reserved worktree, their
    /// results enter the conversation, and their lifecycle is recorded as
    /// ToolStarted/ToolCompleted/ToolFailed. Without this, declared tools
    /// stay propose-only.
    pub fn with_tool_execution(
        mut self,
        executor: Box<dyn tools::ShellToolExecutor>,
        worktree: std::path::PathBuf,
    ) -> Self {
        self.tool_executor = Some(executor);
        self.worktree = Some(worktree);
        self
    }

    #[cfg(test)]
    pub(crate) fn completion_report_for_test(&self) -> Option<String> {
        self.completion_report.clone()
    }

    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    pub fn run_summary(&self) -> LoopRun {
        LoopRun {
            version: NATIVE_LOOP_VERSION,
            dispatch_id: self.binding.dispatch_id.clone(),
            task_id: self.task_id.clone(),
            provider_id: self.provider_id.clone(),
            model_id: self.model.id.clone(),
            turns: self.turns,
            input_tokens_used: self.input_tokens,
            output_tokens_used: self.output_tokens,
            unreported_usage_turns: self.unreported_usage_turns,
            completion_report: self.completion_report.clone(),
            halted: self.halted,
        }
    }

    /// Event ids stay within the 128-byte CommandId bound for any legal
    /// dispatch id; a failure to build a valid event halts rather than drops.
    fn record(&mut self, payload: RuntimeEventKind) -> Result<(), LoopError> {
        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;
        let session_suffix: String = self.binding.session_id.as_str().chars().take(97).collect();
        let id = CommandId::new(format!("e{sequence}_{session_suffix}"))
            .map_err(|_| LoopError::InvalidContract)?;
        match RuntimeEvent::new(id, sequence, self.binding.clone(), payload) {
            Ok(event) => {
                self.events.push(event);
                Ok(())
            }
            Err(_) => {
                self.halted = Some(HaltReason::EnvelopeMismatch);
                Err(LoopError::EnvelopeMismatch)
            }
        }
    }

    /// Runs the turn loop until the model stops, requests completion, or a
    /// bound halts it. Each turn builds a fully validated ProviderRequest
    /// (bounded by the dispatch contract's context, not the model maximum),
    /// calls the transport, validates the envelope via the SDK's own
    /// `ProviderResponse::validate`, and records events.
    pub fn run<T: InferenceTransport + ?Sized>(
        &mut self,
        task_prompt: &str,
        transport: &mut T,
    ) -> Result<LoopRun, LoopError> {
        if self.completion_report.is_some() || self.halted.is_some() {
            return Err(LoopError::AlreadyComplete);
        }
        if task_prompt.trim().is_empty() || task_prompt.len() > 64 * 1024 {
            return Err(LoopError::InvalidInput);
        }
        // The SDK's session consumers require Ready before Message/Exit.
        self.record(RuntimeEventKind::Ready {})?;
        self.messages.push(InputMessage {
            role: MessageRole::User,
            content: vec![InputPart::Text {
                text: task_prompt.to_owned(),
            }],
        });
        loop {
            if self.turns >= MAX_TURNS_PER_RUN {
                self.halted = Some(HaltReason::TurnCapReached);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text("turn cap reached"),
                })?;
                return Err(LoopError::TurnCapReached);
            }
            self.turns += 1;
            let request = self.build_request()?;
            let response = match transport.request(&request, &self.model) {
                Ok(response) => response,
                Err(error) => {
                    self.halted = Some(HaltReason::ProviderFailed);
                    self.record(RuntimeEventKind::Diagnostic {
                        message: truncate_event_text(&format!("provider error: {error}")),
                    })?;
                    return Err(LoopError::ProviderFailed);
                }
            };
            // Full SDK envelope validation: schema version, tool-call
            // coherence, finish/text consistency, duplicate call ids,
            // usage-vs-window. The loop is the enforcement point.
            if let Err(error) = response.validate(&request, &self.model) {
                // The SDK already rejects undeclared tools against the
                // request's tool set; attribute that specifically.
                let (halt, error_code) = if matches!(
                    error,
                    symbiote_runtime_sdk::provider::ProviderError::UnknownTool
                ) {
                    (HaltReason::UnknownTool, LoopError::UnknownTool)
                } else {
                    (HaltReason::EnvelopeMismatch, LoopError::EnvelopeMismatch)
                };
                self.halted = Some(halt);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text(&format!("envelope rejected: {error}")),
                })?;
                return Err(error_code);
            }
            self.accumulate_usage(&response.usage, &request)?;
            self.accumulated_output_bytes = self
                .accumulated_output_bytes
                .saturating_add(response.text.len());
            if self.accumulated_output_bytes > MAX_ACCUMULATED_OUTPUT_BYTES {
                self.halted = Some(HaltReason::OutputBackstop);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text("accumulated output backstop"),
                })?;
                return Err(LoopError::BudgetExhausted);
            }
            if !response.text.is_empty() {
                self.messages.push(InputMessage {
                    role: MessageRole::Assistant,
                    content: vec![InputPart::Text {
                        text: response.text.clone(),
                    }],
                });
                self.record(RuntimeEventKind::Message {
                    text: truncate_event_text(&response.text),
                })?;
            }
            for call in &response.tool_calls {
                if !self.tools.iter().any(|t| t == &call.name) {
                    // A Diagnostic, not ToolFailed: the SDK's session tracker
                    // requires ToolFailed to follow a started tool, and this
                    // proposal is refused before it ever starts.
                    self.halted = Some(HaltReason::UnknownTool);
                    self.record(RuntimeEventKind::Diagnostic {
                        message: truncate_event_text(&format!(
                            "undeclared tool proposed: {}",
                            call.name
                        )),
                    })?;
                    // The loop reports and stops: an undeclared tool is a
                    // contract violation, not something to paper over.
                    return Err(LoopError::UnknownTool);
                }
                self.record(RuntimeEventKind::ToolProposed {
                    tool_call_id: call.call_id.clone(),
                    name: truncate_event_text(&call.name),
                    arguments: truncate_event_text(&call.arguments.to_string()),
                })?;
                // Host-gated tool execution: a proposal the dispatch
                // contract declared AND the Host has an executor for runs
                // through the sandbox and its result enters the
                // conversation; a declared tool with no executor stays
                // propose-only (recorded, never silently dropped).
                if let Some(executor) = self.tool_executor.as_mut() {
                    let invocation = match tools::parse_shell_arguments(&call.arguments) {
                        Ok(invocation) => invocation,
                        Err(error) => {
                            self.halted = Some(HaltReason::EnvelopeMismatch);
                            self.record(RuntimeEventKind::Diagnostic {
                                message: truncate_event_text(&format!(
                                    "tool arguments rejected: {error:?}"
                                )),
                            })?;
                            return Err(LoopError::EnvelopeMismatch);
                        }
                    };
                    let worktree = self.worktree.clone().ok_or(LoopError::UnknownTool)?;
                    let events = match tools::execute_shell_tool(
                        &mut **executor,
                        &call.call_id,
                        &worktree,
                        &invocation,
                    ) {
                        Ok(events) => events,
                        Err(error) => {
                            self.halted = Some(HaltReason::ProviderFailed);
                            self.record(RuntimeEventKind::Diagnostic {
                                message: truncate_event_text(&format!(
                                    "tool execution failed: {error:?}"
                                )),
                            })?;
                            return Err(LoopError::ProviderFailed);
                        }
                    };
                    for event in events {
                        // Both completions AND failures enter the
                        // conversation so the model learns the result — a
                        // failed tool must not be re-executed blind (the
                        // amnesia fix covers failures too).
                        let result_text = match &event {
                            RuntimeEventKind::ToolCompleted { output, .. } => {
                                Some(output.as_str().to_owned())
                            }
                            RuntimeEventKind::ToolFailed { error, .. } => {
                                Some(error.as_str().to_owned())
                            }
                            _ => None,
                        };
                        if let Some(result) = result_text {
                            // Tool results are conversation growth the
                            // provider never reports usage for: charge them
                            // against a dedicated bound so a tool storm
                            // halts with BudgetExhausted instead of
                            // inflating the envelope until it trips.
                            self.tool_result_bytes =
                                self.tool_result_bytes.saturating_add(result.len());
                            if self.tool_result_bytes > MAX_TOOL_RESULT_BYTES {
                                self.halted = Some(HaltReason::BudgetExhausted);
                                self.record(RuntimeEventKind::Diagnostic {
                                    message: truncate_event_text("tool result budget exhausted"),
                                })?;
                                return Err(LoopError::BudgetExhausted);
                            }
                            self.messages.push(InputMessage {
                                role: MessageRole::User,
                                content: vec![InputPart::Text {
                                    text: format!("tool {} result: {}", call.name, result),
                                }],
                            });
                        }
                        self.record(event)?;
                    }
                }
            }
            match response.finish_reason {
                FinishReason::Stop => {
                    self.halted = Some(HaltReason::Stop);
                    self.record(RuntimeEventKind::Exit { code: Some(0) })?;
                    return Ok(self.run_summary());
                }
                FinishReason::Length => {
                    self.halted = Some(HaltReason::Length);
                    self.record(RuntimeEventKind::Diagnostic {
                        message: truncate_event_text("provider length exhaustion"),
                    })?;
                    return Ok(self.run_summary());
                }
                FinishReason::Cancelled => {
                    self.halted = Some(HaltReason::Stop);
                    self.record(RuntimeEventKind::CancelAcknowledged {})?;
                    return Ok(self.run_summary());
                }
                FinishReason::ToolCalls => continue,
            }
        }
    }

    /// A worker (or test harness) explicitly files its completion report;
    /// the Host treats it as RequestCompletion evidence, never as completion.
    pub fn request_completion(&mut self, report: &str) -> Result<(), LoopError> {
        if self.completion_report.is_some() || self.halted.is_some() {
            return Err(LoopError::AlreadyComplete);
        }
        if report.trim().is_empty() || report.len() > MAX_REPORT_BYTES {
            return Err(LoopError::InvalidInput);
        }
        self.completion_report = Some(report.to_owned());
        self.record(RuntimeEventKind::CompletionRequested {
            report: truncate_event_text(report),
        })?;
        Ok(())
    }

    fn build_request(&mut self) -> Result<ProviderRequest, LoopError> {
        self.request_counter += 1;
        let request_id = RequestId::new(format!(
            "req_{}_{}",
            self.binding.session_id.as_str(),
            self.request_counter
        ))
        .map_err(|_| LoopError::InvalidContract)?;
        // The context budget is the dispatch contract's, tightened by what
        // this loop has already consumed; never the model's full window.
        let remaining_input =
            (u64::from(self.contract_context.max_input_tokens)).saturating_sub(self.input_tokens);
        let remaining_output = (u64::from(self.contract_context.reserved_output_tokens))
            .saturating_sub(self.output_tokens);
        if remaining_input == 0 || remaining_output == 0 {
            self.halted = Some(HaltReason::BudgetExhausted);
            self.record(RuntimeEventKind::Diagnostic {
                message: truncate_event_text("dispatch context budget exhausted"),
            })?;
            return Err(LoopError::BudgetExhausted);
        }
        let request = ProviderRequest {
            schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
            request_id,
            provider_id: self.provider_id.clone(),
            model_id: self.model.id.clone(),
            messages: self.messages.clone(),
            tools: self
                .tools
                .iter()
                .map(|name| symbiote_runtime_sdk::provider::ToolDefinition {
                    name: name.clone(),
                    description: String::new(),
                    input_schema: serde_json::json!({"type": "object"}),
                })
                .collect(),
            input_token_budget: remaining_input.min(self.model.context_window_tokens),
            max_output_tokens: remaining_output.min(self.model.max_output_tokens),
            reasoning: None,
            streaming: false,
        };
        if let Err(error) = request.validate(&self.model) {
            self.halted = Some(HaltReason::EnvelopeMismatch);
            self.record(RuntimeEventKind::Diagnostic {
                message: truncate_event_text(&format!("request rejected: {error}")),
            })?;
            return Err(LoopError::EnvelopeMismatch);
        }
        Ok(request)
    }

    /// Conservative accounting. Known usage splits input/output exactly.
    /// Unknown usage is never free: the turn's full request budgets are
    /// charged (the provider may have processed all of it), the turn counts
    /// as unreported, and the accumulated-output byte backstop bounds
    /// runaway loops that report nothing.
    fn accumulate_usage(
        &mut self,
        usage: &TokenUsage,
        request: &ProviderRequest,
    ) -> Result<(), LoopError> {
        match usage.total() {
            Ok(Some(_)) => {
                if let TokenUsage::Known {
                    input_tokens,
                    output_tokens,
                    ..
                } = usage
                {
                    self.input_tokens = self.input_tokens.saturating_add(*input_tokens);
                    self.output_tokens = self.output_tokens.saturating_add(*output_tokens);
                }
            }
            Ok(None) => {
                self.unreported_usage_turns += 1;
                self.input_tokens = self.input_tokens.saturating_add(request.input_token_budget);
                self.output_tokens = self.output_tokens.saturating_add(request.max_output_tokens);
            }
            Err(_) => {
                self.halted = Some(HaltReason::EnvelopeMismatch);
                self.record(RuntimeEventKind::Diagnostic {
                    message: truncate_event_text("inconsistent usage envelope"),
                })?;
                return Err(LoopError::EnvelopeMismatch);
            }
        }
        if self.output_tokens > u64::from(self.contract_context.reserved_output_tokens)
            || self.input_tokens > u64::from(self.contract_context.max_input_tokens)
        {
            self.halted = Some(HaltReason::BudgetExhausted);
            self.record(RuntimeEventKind::Diagnostic {
                message: truncate_event_text("dispatch context budget exhausted"),
            })?;
            return Err(LoopError::BudgetExhausted);
        }
        Ok(())
    }
}

/// EventText permits 16 KiB; transport text may legally be larger inside a
/// 256 KiB envelope, so event payloads truncate on a char boundary instead
/// of panicking. Truncation is visible (the marker), never silent.
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
    use std::collections::BTreeSet;
    use symbiote_runtime_sdk::provider::ProviderToolCall;

    fn model() -> ModelDescriptor {
        ModelDescriptor {
            schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
            id: ModelId::new("model-fixture").unwrap(),
            provider_id: ProviderConnectionId::new("native").unwrap(),
            context_window_tokens: 8192,
            max_output_tokens: 4096,
            capabilities: symbiote_runtime_sdk::provider::ModelCapabilities {
                reasoning_efforts: BTreeSet::new(),
                tools: true,
                images: false,
                streaming: false,
            },
        }
    }

    fn dispatch(task: &Task) -> Dispatch {
        let host_id = HostId::new("host").unwrap();
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
            required_tools: BTreeSet::from(["edit_file".to_string(), "shell".to_string()]),
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

    fn stop_response(request: &ProviderRequest, text: &str) -> ProviderResponse {
        ProviderResponse {
            schema_version: request.schema_version,
            request_id: request.request_id.clone(),
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            text: text.to_owned(),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage::Known {
                input_tokens: 10,
                output_tokens: 10,
                cached_input_tokens: None,
                reasoning_tokens: None,
            },
        }
    }

    #[test]
    fn single_turn_stop_runs_within_budget_and_records_events() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        let mut transport = ScriptedTransport::new(Vec::new());
        // Empty script falls back to a valid Stop envelope with text.
        let run = session
            .run("Implement the fixture", &mut transport)
            .unwrap();
        assert_eq!(run.turns, 1);
        assert_eq!(run.halted, Some(HaltReason::Stop));
        // Ready precedes every other event (tracker-ingestible order);
        // then the assistant Message and the Exit.
        assert_eq!(session.events().len(), 3);
        assert!(matches!(
            session.events()[0].payload(),
            RuntimeEventKind::Ready {}
        ));
        // The request honored the contract budget, not the model window.
        let request = &transport.seen_requests()[0];
        assert!(request.input_token_budget <= 500);
        assert!(request.max_output_tokens <= 300);
        assert_eq!(
            request
                .tools
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>(),
            vec!["edit_file".to_string(), "shell".to_string()]
        );
        // The summary carries the real task and provider identities.
        assert_eq!(run.task_id.as_str(), "task");
        assert_eq!(run.provider_id.as_str(), "native");
    }

    #[test]
    fn tool_calls_continue_until_stop_and_undeclared_tools_halt() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        struct TwoTurn {
            first: bool,
        }
        impl InferenceTransport for TwoTurn {
            fn request(
                &mut self,
                request: &ProviderRequest,
                _model: &ModelDescriptor,
            ) -> Result<ProviderResponse, ProviderError> {
                if self.first {
                    self.first = false;
                    Ok(ProviderResponse {
                        schema_version: request.schema_version,
                        request_id: request.request_id.clone(),
                        provider_id: request.provider_id.clone(),
                        model_id: request.model_id.clone(),
                        text: "Editing".into(),
                        tool_calls: vec![ProviderToolCall {
                            call_id: RequestId::new("call-1").unwrap(),
                            name: "edit_file".into(),
                            arguments: serde_json::json!({"path": "a.rs"}),
                        }],
                        finish_reason: FinishReason::ToolCalls,
                        usage: TokenUsage::Known {
                            input_tokens: 10,
                            output_tokens: 10,
                            cached_input_tokens: None,
                            reasoning_tokens: None,
                        },
                    })
                } else {
                    Ok(stop_response(request, "Done"))
                }
            }
        }
        let mut transport = TwoTurn { first: true };
        let run = session.run("Implement", &mut transport).unwrap();
        assert_eq!(run.turns, 2);
        // Ready + Message + ToolProposed + Message + Exit.
        assert_eq!(session.events().len(), 5);

        // Undeclared tool halts with UnknownTool and a Diagnostic event.
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        struct Rogue;
        impl InferenceTransport for Rogue {
            fn request(
                &mut self,
                request: &ProviderRequest,
                _model: &ModelDescriptor,
            ) -> Result<ProviderResponse, ProviderError> {
                Ok(ProviderResponse {
                    schema_version: request.schema_version,
                    request_id: request.request_id.clone(),
                    provider_id: request.provider_id.clone(),
                    model_id: request.model_id.clone(),
                    text: "working".into(),
                    tool_calls: vec![ProviderToolCall {
                        call_id: RequestId::new("call-rogue").unwrap(),
                        name: "rogue_tool".into(),
                        arguments: serde_json::json!({}),
                    }],
                    finish_reason: FinishReason::ToolCalls,
                    usage: TokenUsage::Known {
                        input_tokens: 5,
                        output_tokens: 5,
                        cached_input_tokens: None,
                        reasoning_tokens: None,
                    },
                })
            }
        }
        let mut rogue = Rogue;
        assert_eq!(
            session.run("Implement", &mut rogue).unwrap_err(),
            LoopError::UnknownTool
        );
        assert_eq!(session.run_summary().halted, Some(HaltReason::UnknownTool));
        // The refusal is a Diagnostic the SDK tracker accepts.
        assert!(matches!(
            session.events().last().map(|e| e.payload()),
            Some(RuntimeEventKind::Diagnostic { .. })
        ));
    }

    #[test]
    fn shell_tools_execute_through_the_injected_executor() {
        // Turn 1 proposes a shell tool; the session has tool execution
        // enabled with a scripted executor. The tool runs, its lifecycle is
        // recorded (ToolProposed → ToolStarted → ToolCompleted), the result
        // enters the conversation, and turn 2's request carries the tool
        // result text. Turn 2 then stops with a summary message.
        let task = task();
        let dispatch = dispatch(&task);
        let session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        struct TwoTurn {
            first: bool,
        }
        impl InferenceTransport for TwoTurn {
            fn request(
                &mut self,
                request: &ProviderRequest,
                _model: &ModelDescriptor,
            ) -> Result<ProviderResponse, ProviderError> {
                if self.first {
                    self.first = false;
                    Ok(ProviderResponse {
                        schema_version: request.schema_version,
                        request_id: request.request_id.clone(),
                        provider_id: request.provider_id.clone(),
                        model_id: request.model_id.clone(),
                        text: "Running tests".into(),
                        tool_calls: vec![ProviderToolCall {
                            call_id: RequestId::new("call-shell").unwrap(),
                            name: "shell".into(),
                            arguments: serde_json::json!({
                                "program": "cargo",
                                "arguments": ["test", "--locked"],
                            }),
                        }],
                        finish_reason: FinishReason::ToolCalls,
                        usage: TokenUsage::Known {
                            input_tokens: 10,
                            output_tokens: 10,
                            cached_input_tokens: None,
                            reasoning_tokens: None,
                        },
                    })
                } else {
                    // The second request's messages must carry the tool
                    // result; assert before responding.
                    assert!(
                        request
                            .messages
                            .iter()
                            .any(|m| m.content.iter().any(
                                |p| matches!(p, InputPart::Text { text } if text.contains("shell result: test result: ok"))
                            )),
                        "the tool result must enter the conversation"
                    );
                    Ok(stop_response(request, "Done"))
                }
            }
        }
        let mut transport = TwoTurn { first: true };
        struct FixedExecutor;
        impl tools::ShellToolExecutor for FixedExecutor {
            fn run_shell(
                &mut self,
                _invocation: &tools::ShellInvocation,
                _worktree: &std::path::Path,
            ) -> Result<(Vec<u8>, Option<i32>), tools::ToolExecError> {
                Ok((b"test result: ok".to_vec(), Some(0)))
            }
        }
        let executor: Box<dyn tools::ShellToolExecutor> = Box::new(FixedExecutor);
        let worktree =
            std::env::temp_dir().join(format!("symbiote-shelltest-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&worktree);
        let mut session = session.with_tool_execution(executor, worktree);
        let run = session.run("Run the tests", &mut transport).unwrap();
        assert_eq!(run.turns, 2);
        // Ready, Message(Running tests), ToolProposed, ToolStarted,
        // ToolCompleted, Message(Done), Exit.
        let kinds: Vec<_> = session.events().iter().map(|e| e.payload()).collect();
        assert!(matches!(kinds[0], RuntimeEventKind::Ready {}));
        assert!(kinds.iter().any(|k| matches!(
            k,
            RuntimeEventKind::ToolProposed { name, .. } if name.as_str() == "shell"
        )));
        assert!(kinds.iter().any(|k| matches!(
            k,
            RuntimeEventKind::ToolStarted { tool_call_id } if tool_call_id.as_str() == "call-shell"
        )));
        assert!(kinds.iter().any(|k| matches!(
            k,
            RuntimeEventKind::ToolCompleted { tool_call_id, .. } if tool_call_id.as_str() == "call-shell"
        )));
        assert!(matches!(
            kinds.last().unwrap(),
            RuntimeEventKind::Exit { code: Some(0) }
        ));
    }

    #[test]
    fn budget_exhaustion_halts_before_the_provider_call() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        // Simulate an already-consumed budget.
        session.output_tokens = 300; // equals reserved_output_tokens
        let mut transport = ScriptedTransport::new(Vec::new());
        assert_eq!(
            session.run("Implement", &mut transport).unwrap_err(),
            LoopError::BudgetExhausted
        );
        assert!(transport.seen_requests().is_empty());
    }

    #[test]
    fn unreported_usage_charges_conservatively_and_halts() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        struct Silent;
        impl InferenceTransport for Silent {
            fn request(
                &mut self,
                request: &ProviderRequest,
                _model: &ModelDescriptor,
            ) -> Result<ProviderResponse, ProviderError> {
                Ok(ProviderResponse {
                    schema_version: request.schema_version,
                    request_id: request.request_id.clone(),
                    provider_id: request.provider_id.clone(),
                    model_id: request.model_id.clone(),
                    text: "chunk".into(),
                    tool_calls: vec![ProviderToolCall {
                        call_id: RequestId::new(format!("call-{}", request.request_id.as_str()))
                            .unwrap(),
                        name: "edit_file".into(),
                        arguments: serde_json::json!({}),
                    }],
                    finish_reason: FinishReason::ToolCalls,
                    usage: TokenUsage::Unknown {
                        reason: symbiote_runtime_sdk::provider::UsageUnknownReason::NotReported,
                    },
                })
            }
        }
        // One unreported turn charges the full request budgets (500/300),
        // exhausting the contract immediately — never 64 free turns.
        let mut silent = Silent;
        assert_eq!(
            session.run("Implement", &mut silent).unwrap_err(),
            LoopError::BudgetExhausted
        );
        let summary = session.run_summary();
        assert_eq!(summary.unreported_usage_turns, 1);
        assert!(summary.input_tokens_used >= 500);
        assert_eq!(summary.halted, Some(HaltReason::BudgetExhausted));
    }

    #[test]
    fn completion_report_is_evidence_never_completion() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        session
            .request_completion("Changed a.rs; tests pass")
            .unwrap();
        assert_eq!(
            session.completion_report_for_test(),
            Some("Changed a.rs; tests pass".to_string())
        );
        // The event stream recorded the worker's request for the Host.
        assert!(matches!(
            session.events().last().map(|e| e.payload()),
            Some(RuntimeEventKind::CompletionRequested { .. })
        ));
        // Double filing is refused.
        assert_eq!(
            session.request_completion("again").unwrap_err(),
            LoopError::AlreadyComplete
        );
        // Oversized reports are typed errors, never panics.
        let oversized = "x".repeat(MAX_REPORT_BYTES + 1);
        let mut fresh = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        assert_eq!(
            fresh.request_completion(&oversized).unwrap_err(),
            LoopError::InvalidInput
        );
    }

    #[test]
    fn expired_contract_refuses_session_creation() {
        let task = task();
        let dispatch = dispatch(&task);
        // Enforcement claims expire at Timestamp(1_000_000); past that the
        // session cannot even be created.
        assert!(matches!(
            NativeSession::new(&dispatch, model(), Timestamp(2_000_000)),
            Err(LoopError::InvalidContract)
        ));
    }

    #[test]
    fn invalid_envelopes_are_rejected_by_sdk_validation() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        struct BadEnvelope;
        impl InferenceTransport for BadEnvelope {
            fn request(
                &mut self,
                request: &ProviderRequest,
                _model: &ModelDescriptor,
            ) -> Result<ProviderResponse, ProviderError> {
                let mut response = stop_response(request, "hi");
                // Stop with tool calls attached violates SDK coherence.
                response.tool_calls = vec![ProviderToolCall {
                    call_id: RequestId::new("call-bad").unwrap(),
                    name: "edit_file".into(),
                    arguments: serde_json::json!({}),
                }];
                Ok(response)
            }
        }
        let mut bad = BadEnvelope;
        assert_eq!(
            session.run("Implement", &mut bad).unwrap_err(),
            LoopError::EnvelopeMismatch
        );
        assert_eq!(
            session.run_summary().halted,
            Some(HaltReason::EnvelopeMismatch)
        );
    }
}
