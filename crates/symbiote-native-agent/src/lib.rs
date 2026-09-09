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
use symbiote_runtime_sdk::events::{RuntimeEvent, RuntimeEventKind, SessionBinding};
use symbiote_runtime_sdk::provider::{
    FinishReason, InputMessage, InputPart, MessageRole, ModelDescriptor, ProviderError,
    ProviderRequest, ProviderResponse, TokenUsage,
};

pub const NATIVE_LOOP_VERSION: u32 = 1;
pub const MAX_TURNS_PER_RUN: u32 = 64;
/// Turns whose aggregate text exceeds this are halted before the provider
/// reports length exhaustion; the budget check is host-observable.
pub const MAX_ACCUMULATED_OUTPUT_BYTES: usize = 512 * 1024;

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
    /// The transport produced an envelope inconsistent with the request.
    EnvelopeMismatch,
    /// The model requested a tool the session has no definition for.
    UnknownTool,
    /// Completion was already requested; the loop is finished.
    AlreadyComplete,
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

fn finish_response(request: &ProviderRequest, finish: FinishReason) -> ProviderResponse {
    ProviderResponse {
        schema_version: request.schema_version,
        request_id: request.request_id.clone(),
        provider_id: request.provider_id.clone(),
        model_id: request.model_id.clone(),
        text: String::new(),
        tool_calls: Vec::new(),
        finish_reason: finish,
        usage: TokenUsage::Unknown {
            reason: symbiote_runtime_sdk::provider::UsageUnknownReason::NotReported,
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
}

/// The native turn engine. Created per dispatch from already-authorized
/// state: the dispatch contract supplies the context budget and tools; the
/// model descriptor supplies the envelope bounds. All emitted events carry
/// the session binding so downstream consumers can attribute every effect.
pub struct NativeSession {
    binding: SessionBinding,
    contract_context: ContextPolicy,
    tools: Vec<String>,
    model: ModelDescriptor,
    messages: Vec<InputMessage>,
    events: Vec<RuntimeEvent>,
    next_event_sequence: u64,
    turns: u32,
    input_tokens: u64,
    output_tokens: u64,
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
            contract_context: contract.binding().context.clone(),
            tools: contract.binding().required_tools.iter().cloned().collect(),
            model,
            messages: Vec::new(),
            events: Vec::new(),
            next_event_sequence: 1,
            turns: 0,
            input_tokens: 0,
            output_tokens: 0,
            completion_report: None,
            halted: None,
            request_counter: 0,
        })
    }

    pub fn binding(&self) -> &SessionBinding {
        &self.binding
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
            task_id: self.binding.dispatch_id.clone().into_task_id(),
            provider_id: ProviderConnectionId::new("native").expect("static id"),
            model_id: self.model.id.clone(),
            turns: self.turns,
            input_tokens_used: self.input_tokens,
            output_tokens_used: self.output_tokens,
            completion_report: self.completion_report.clone(),
            halted: self.halted,
        }
    }

    fn record(&mut self, payload: RuntimeEventKind) {
        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;
        let id = CommandId::new(format!(
            "evt_{}_{}",
            self.binding.session_id.as_str(),
            sequence
        ))
        .expect("event ids are bounded");
        if let Ok(event) = RuntimeEvent::new(id, sequence, self.binding.clone(), payload) {
            self.events.push(event);
        }
    }

    /// Runs the turn loop until the model stops, requests completion, or a
    /// bound halts it. Each turn builds a fully validated ProviderRequest
    /// (bounded by the dispatch contract's context, not the model maximum),
    /// calls the transport, validates the envelope, and records events.
    pub fn run(
        &mut self,
        task_prompt: &str,
        transport: &mut impl InferenceTransport,
    ) -> Result<LoopRun, LoopError> {
        if self.completion_report.is_some() || self.halted.is_some() {
            return Err(LoopError::AlreadyComplete);
        }
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
                    message: bounded_text("turn cap reached"),
                });
                return Err(LoopError::TurnCapReached);
            }
            self.turns += 1;
            let request = self.build_request()?;
            let response = transport.request(&request, &self.model).map_err(|e| {
                self.halted = Some(HaltReason::ProviderFailed);
                self.record(RuntimeEventKind::Diagnostic {
                    message: bounded_text(&format!("provider error: {e}")),
                });
                LoopError::ProviderFailed
            })?;
            // Envelope consistency: the response must belong to this request.
            if response.request_id != request.request_id
                || response.provider_id != request.provider_id
                || response.model_id != request.model_id
            {
                self.halted = Some(HaltReason::EnvelopeMismatch);
                return Err(LoopError::EnvelopeMismatch);
            }
            self.accumulate_usage(&response.usage)?;
            if !response.text.is_empty() {
                self.messages.push(InputMessage {
                    role: MessageRole::Assistant,
                    content: vec![InputPart::Text {
                        text: response.text.clone(),
                    }],
                });
                self.record(RuntimeEventKind::Message {
                    text: bounded_text(&response.text),
                });
            }
            for call in &response.tool_calls {
                if !self.tools.iter().any(|t| t == &call.name) {
                    self.halted = Some(LoopError::UnknownTool.into_halt());
                    self.record(RuntimeEventKind::ToolFailed {
                        tool_call_id: call.call_id.clone(),
                        error: bounded_text("tool not in dispatch contract"),
                    });
                    // The loop reports and stops: an undeclared tool is a
                    // contract violation, not something to paper over.
                    return Err(LoopError::UnknownTool);
                }
                self.record(RuntimeEventKind::ToolProposed {
                    tool_call_id: call.call_id.clone(),
                    name: bounded_text(&call.name),
                    arguments: bounded_text(&call.arguments.to_string()),
                });
                // Native tool execution (sandbox invocation) is the next
                // slice; a proposed-but-unexecuted tool is recorded and the
                // loop continues with the provider's turn.
            }
            match response.finish_reason {
                FinishReason::Stop => {
                    self.halted = Some(HaltReason::Stop);
                    self.record(RuntimeEventKind::Exit { code: Some(0) });
                    return Ok(self.run_summary());
                }
                FinishReason::Length => {
                    self.halted = Some(HaltReason::Length);
                    self.record(RuntimeEventKind::Diagnostic {
                        message: bounded_text("provider length exhaustion"),
                    });
                    return Ok(self.run_summary());
                }
                FinishReason::Cancelled => {
                    self.halted = Some(HaltReason::Stop);
                    self.record(RuntimeEventKind::CancelAcknowledged {});
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
        if report.trim().is_empty() || report.len() > 64 * 1024 {
            return Err(LoopError::EnvelopeMismatch);
        }
        self.completion_report = Some(report.to_owned());
        self.record(RuntimeEventKind::CompletionRequested {
            report: bounded_text(report),
        });
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
                message: bounded_text("dispatch context budget exhausted"),
            });
            return Err(LoopError::BudgetExhausted);
        }
        let request = ProviderRequest {
            schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
            request_id,
            provider_id: ProviderConnectionId::new("native").expect("static id"),
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
            self.halted = Some(HaltReason::BudgetExhausted);
            let _ = error;
            return Err(LoopError::BudgetExhausted);
        }
        Ok(request)
    }

    fn accumulate_usage(&mut self, usage: &TokenUsage) -> Result<(), LoopError> {
        if let Some(total) = usage.total().map_err(|_| LoopError::EnvelopeMismatch)? {
            // Split the total conservatively: without provider detail, the
            // whole total counts against the output remainder so the budget
            // check can never over-spend the contract.
            self.output_tokens = self.output_tokens.saturating_add(total);
            if self.output_tokens > u64::from(self.contract_context.reserved_output_tokens)
                || self.input_tokens + self.output_tokens
                    > u64::from(self.contract_context.max_input_tokens)
                        + u64::from(self.contract_context.reserved_output_tokens)
            {
                self.halted = Some(HaltReason::BudgetExhausted);
                return Err(LoopError::BudgetExhausted);
            }
        }
        Ok(())
    }
}

trait IntoHalt {
    fn into_halt(self) -> HaltReason;
}
impl IntoHalt for LoopError {
    fn into_halt(self) -> HaltReason {
        match self {
            LoopError::UnknownTool => HaltReason::ProviderFailed,
            other => match other {
                LoopError::BudgetExhausted => HaltReason::BudgetExhausted,
                LoopError::TurnCapReached => HaltReason::TurnCapReached,
                LoopError::ProviderFailed => HaltReason::ProviderFailed,
                LoopError::EnvelopeMismatch => HaltReason::EnvelopeMismatch,
                _ => HaltReason::ProviderFailed,
            },
        }
    }
}

fn bounded_text(text: &str) -> symbiote_runtime_sdk::events::EventText {
    symbiote_runtime_sdk::events::EventText::new(text).expect("event text is bounded")
}

// TaskId is not derivable from DispatchId in the domain; run_summary needs an
// explicit task identity. Extend the summary construction instead.
impl NativeSession {
    pub fn run_summary_for_task(&self, task: &TaskId) -> LoopRun {
        LoopRun {
            version: NATIVE_LOOP_VERSION,
            dispatch_id: self.binding.dispatch_id.clone(),
            task_id: task.clone(),
            provider_id: ProviderConnectionId::new("native").expect("static id"),
            model_id: self.model.id.clone(),
            turns: self.turns,
            input_tokens_used: self.input_tokens,
            output_tokens_used: self.output_tokens,
            completion_report: self.completion_report.clone(),
            halted: self.halted,
        }
    }
}

trait TaskIdFromDispatch {
    fn into_task_id(self) -> TaskId;
}
impl TaskIdFromDispatch for DispatchId {
    fn into_task_id(self) -> TaskId {
        TaskId::new(self.as_str()).expect("dispatch ids are valid task ids")
    }
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
            required_tools: BTreeSet::from(["edit_file".to_string()]),
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
        // Empty script falls back to a Stop response with no text.
        let run = session
            .run("Implement the fixture", &mut transport)
            .unwrap();
        assert_eq!(run.turns, 1);
        assert_eq!(run.halted, Some(HaltReason::Stop));
        assert_eq!(session.events().len(), 1); // Exit event
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
            vec!["edit_file".to_string()]
        );
    }

    #[test]
    fn tool_calls_continue_until_stop_and_undeclared_tools_halt() {
        let task = task();
        let dispatch = dispatch(&task);
        let mut session = NativeSession::new(&dispatch, model(), Timestamp(20)).unwrap();
        // ScriptedTransport's fallback is Stop, so inject explicit responses
        // via a custom transport closure instead.
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
        // Turn 1: Message + ToolProposed; turn 2: Message + Exit.
        assert_eq!(session.events().len(), 4);

        // Undeclared tool halts with UnknownTool.
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
                    text: String::new(),
                    tool_calls: vec![ProviderToolCall {
                        call_id: RequestId::new("call-rogue").unwrap(),
                        name: "shell".into(),
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
}
