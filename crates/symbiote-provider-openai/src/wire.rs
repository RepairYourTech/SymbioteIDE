//! The provider wire mapping: the neutral request encoded onto the provider's
//! chat-completions shape, and the provider's answer — whole, or streamed as
//! server-sent events — decoded back into the neutral contract.
//!
//! Encoding is total and boring on purpose: messages in order with their text
//! parts joined, tools in their function form, the output budget as `max_tokens`,
//! the effort as `reasoning_effort` when one was requested, and `stream` with its
//! usage option set when the caller asked to stream. The neutral contract already
//! bounds and validated every field; this layer adds no opinion of its own, so a
//! caller cannot smuggle a provider field the contract never carried.
//!
//! Decoding is where the provider's dialects are absorbed. A whole response's
//! `finish_reason` is mapped — `stop`, `tool_calls`, `length` — and anything the
//! adapter does not recognize is refused as an envelope it cannot hold, because a
//! finish reason nobody named is a contract the provider changed. Tool calls
//! arrive with the provider's own ids; those become `RequestId`s, which bounds
//! their shape and makes a duplicate id the duplicate it is. Usage preserves the
//! provider's semantics: prompt and completion totals stand as reported, cached
//! tokens ride beside input and reasoning tokens beside output — subsets
//! recorded, never counted in — and a response without a usage object decodes to
//! `TokenUsage::Unknown` with the reason it is unknown rather than to a zero,
//! because a missing measurement is not a measurement of nothing.
//!
//! Streaming is an `StreamAssembler`: the server's events are fed to it line by
//! line, and it accumulates the deltas — text, and tool-call arguments that
//! arrive fragmented across chunks and are joined per call — until the provider
//! names its finish reason. The assembler holds no opinion about transport
//! failures: a stream that ends without a finish reason is the transport's to
//! refuse, because an interrupted generation is not a completed one and must not
//! be handed to the loop as if it were.
use serde_json::{Value, json};
use symbiote_domain::RequestId;
use symbiote_runtime_sdk::provider::{
    FinishReason, MessageRole, PROVIDER_CONTRACT_VERSION, ProviderError, ProviderRequest,
    ProviderResponse, ProviderToolCall, ReasoningEffort, TokenUsage, UsageUnknownReason,
};

/// The neutral request as the provider's JSON. No field here is invented:
/// everything encoded is something the contract carried — and a request this
/// encoding cannot spell honestly is refused, never rewritten: an image part
/// is `UnsupportedCapability` (this adapter is text-only, and dropping the
/// picture while keeping the caption is the coercion this crate refuses),
/// and an effort the wire has no exact spelling for is refused rather than
/// sent as the nearest one.
pub fn encode_request(request: &ProviderRequest) -> Result<Value, ProviderError> {
    for message in &request.messages {
        if message.content.iter().any(|part| {
            matches!(
                part,
                symbiote_runtime_sdk::provider::InputPart::Image { .. }
            )
        }) {
            return Err(ProviderError::UnsupportedCapability);
        }
    }
    let messages: Vec<Value> = request
        .messages
        .iter()
        .map(|message| {
            let role = match message.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            let text = message
                .content
                .iter()
                .filter_map(|part| match part {
                    symbiote_runtime_sdk::provider::InputPart::Text { text } => Some(text.as_str()),
                    symbiote_runtime_sdk::provider::InputPart::Image { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("");
            json!({ "role": role, "content": text })
        })
        .collect();
    let tools: Vec<Value> = request
        .tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.input_schema,
                },
            })
        })
        .collect();
    let mut encoded = json!({
        "model": request.model_id.as_str(),
        "messages": messages,
        "max_tokens": request.max_output_tokens,
        "stream": request.streaming,
    });
    if !tools.is_empty() {
        encoded["tools"] = Value::Array(tools);
    }
    if let Some(effort) = &request.reasoning {
        encoded["reasoning_effort"] = encode_effort(effort)?;
    }
    if request.streaming {
        encoded["stream_options"] = json!({ "include_usage": true });
    }
    Ok(encoded)
}

fn encode_effort(effort: &ReasoningEffort) -> Result<Value, ProviderError> {
    // Exact spellings only. The contract's highest efforts have no
    // provider-wide spelling on this wire, and sending the nearest one would
    // be the silent downgrade that turns a run's answer into a different
    // run's — so they are refused here, wherever they might have slipped
    // past a catalog that advertised them.
    match effort {
        ReasoningEffort::None => Ok(json!("none")),
        ReasoningEffort::Minimal => Ok(json!("minimal")),
        ReasoningEffort::Low => Ok(json!("low")),
        ReasoningEffort::Medium => Ok(json!("medium")),
        ReasoningEffort::High => Ok(json!("high")),
        ReasoningEffort::XHigh | ReasoningEffort::Max | ReasoningEffort::Ultra => {
            Err(ProviderError::UnsupportedCapability)
        }
    }
}

/// One provider tool call decoded: its id made a `RequestId` (the neutral
/// vocabulary bounds what an id can be, and a call whose id is not one is an
/// envelope refused), its arguments parsed as the JSON object the contract
/// carries.
fn decode_tool_call(call: &Value) -> Result<ProviderToolCall, ProviderError> {
    let id = call
        .get("id")
        .and_then(Value::as_str)
        .ok_or(ProviderError::InvalidEnvelope)?;
    let function = call.get("function").ok_or(ProviderError::InvalidEnvelope)?;
    let name = function
        .get("name")
        .and_then(Value::as_str)
        .ok_or(ProviderError::InvalidEnvelope)?;
    let arguments = function
        .get("arguments")
        .and_then(Value::as_str)
        .ok_or(ProviderError::InvalidEnvelope)?;
    Ok(ProviderToolCall {
        call_id: RequestId::new(id).map_err(|_| ProviderError::InvalidEnvelope)?,
        name: name.to_owned(),
        arguments: serde_json::from_str(arguments).map_err(|_| ProviderError::InvalidEnvelope)?,
    })
}

fn decode_usage(usage: Option<&Value>) -> Result<TokenUsage, ProviderError> {
    let Some(usage) = usage else {
        return Ok(TokenUsage::Unknown {
            reason: UsageUnknownReason::NotReported,
        });
    };
    if usage.is_null() {
        return Ok(TokenUsage::Unknown {
            reason: UsageUnknownReason::NotReported,
        });
    }
    let field = |name: &str| usage.get(name).and_then(Value::as_u64);
    match (field("prompt_tokens"), field("completion_tokens")) {
        (Some(input), Some(output)) => {
            let cached = usage
                .get("prompt_tokens_details")
                .and_then(|details| details.get("cached_tokens"))
                .and_then(Value::as_u64);
            let reasoning = usage
                .get("completion_tokens_details")
                .and_then(|details| details.get("reasoning_tokens"))
                .and_then(Value::as_u64);
            Ok(TokenUsage::Known {
                input_tokens: input,
                output_tokens: output,
                cached_input_tokens: cached,
                reasoning_tokens: reasoning,
            })
        }
        // A usage object missing either total is not a usable measurement.
        _ => Ok(TokenUsage::Unknown {
            reason: UsageUnknownReason::NotReported,
        }),
    }
}

fn decode_finish(reason: Option<&str>) -> Result<FinishReason, ProviderError> {
    match reason {
        Some("stop") => Ok(FinishReason::Stop),
        Some("tool_calls") => Ok(FinishReason::ToolCalls),
        Some("length") => Ok(FinishReason::Length),
        _ => Err(ProviderError::InvalidEnvelope),
    }
}

/// A whole (non-streamed) provider response decoded into the neutral
/// contract, validated before it is handed back.
pub fn decode_response(
    body: &Value,
    request: &ProviderRequest,
    model: &symbiote_runtime_sdk::provider::ModelDescriptor,
) -> Result<ProviderResponse, ProviderError> {
    let choice = body
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(ProviderError::InvalidEnvelope)?;
    let message = choice
        .get("message")
        .ok_or(ProviderError::InvalidEnvelope)?;
    let text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut tool_calls = Vec::new();
    match message.get("tool_calls") {
        None | Some(Value::Null) => {}
        Some(Value::Array(calls)) => {
            for call in calls {
                tool_calls.push(decode_tool_call(call)?);
            }
        }
        // A present-but-wrong-typed calls field is data the provider sent
        // and this decode would otherwise drop; a malformed call that
        // vanishes is a call the loop never answers.
        Some(_) => return Err(ProviderError::InvalidEnvelope),
    }
    let finish = decode_finish(choice.get("finish_reason").and_then(Value::as_str))?;
    let response = ProviderResponse {
        schema_version: PROVIDER_CONTRACT_VERSION,
        request_id: request.request_id.clone(),
        provider_id: request.provider_id.clone(),
        model_id: request.model_id.clone(),
        text,
        tool_calls,
        finish_reason: finish,
        usage: decode_usage(body.get("usage"))?,
    };
    response.validate(request, model)?;
    Ok(response)
}

/// Accumulates one streamed response: events in, one neutral response out.
///
/// The assembler absorbs the provider's fragmentation rules so the loop never
/// sees them: text arrives as `delta.content` pieces, a tool call arrives
/// first as its id and name and then as argument fragments carried by its
/// index, and the finish reason and usage arrive in the last chunks. A call
/// index seen after its id was already set continues that call; an argument
/// fragment for an index nobody opened is an envelope refused. Duplicate ids
/// across calls are the duplicate they are.
#[derive(Default)]
pub struct StreamAssembler {
    text: String,
    calls: Vec<StreamCall>,
    finish: Option<FinishReason>,
    usage: Option<TokenUsage>,
}

struct StreamCall {
    id: String,
    name: String,
    arguments: String,
    complete: bool,
}

impl StreamAssembler {
    /// Feeds one `data:` event's JSON payload (the `data:` prefix and the
    /// `[DONE]` sentinel are the transport's to strip).
    pub fn feed(&mut self, event: &Value) -> Result<(), ProviderError> {
        // The provider's usage chunk arrives AFTER the finish chunk — an
        // empty choices list carrying only the usage object — so an event
        // past the finish is refused unless it is exactly that one.
        if self.finish.is_some() {
            let usage_only = event.get("usage").is_some()
                && event
                    .get("choices")
                    .and_then(Value::as_array)
                    .is_none_or(|choices| choices.is_empty());
            if !usage_only {
                return Err(ProviderError::InvalidEnvelope);
            }
            if self.usage.is_none() {
                self.usage = Some(decode_usage(event.get("usage"))?);
            }
            return Ok(());
        }
        if let Some(delta) = event
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("delta"))
        {
            if let Some(piece) = delta.get("content").and_then(Value::as_str) {
                self.text.push_str(piece);
            }
            if let Some(fragments) = delta.get("tool_calls").and_then(Value::as_array) {
                for fragment in fragments {
                    self.feed_tool_fragment(fragment)?;
                }
            }
        }
        // After the first chunk the provider may stream an empty choices list
        // carrying only usage; the delta read above then sees nothing, and the
        // finish reason is looked up on the choice that names it.
        if let Some(reason) = event
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| {
                choices
                    .iter()
                    .find_map(|choice| choice.get("finish_reason"))
            })
            .and_then(Value::as_str)
        {
            self.finish = Some(decode_finish(Some(reason))?);
        }
        // A `"usage": null` in an early chunk is the provider saying "not in
        // this one", not a measurement of nothing — the slot stays open for
        // the usage chunk the stream options asked for.
        if self.usage.is_none() && event.get("usage").is_some_and(|usage| !usage.is_null()) {
            self.usage = Some(decode_usage(event.get("usage"))?);
        }
        Ok(())
    }

    fn feed_tool_fragment(&mut self, fragment: &Value) -> Result<(), ProviderError> {
        let index = fragment
            .get("index")
            .and_then(Value::as_u64)
            .ok_or(ProviderError::InvalidEnvelope)? as usize;
        if fragment.get("id").and_then(Value::as_str).is_some() {
            if self
                .calls
                .get(index)
                .is_some_and(|call| !call.id.is_empty())
                || index != self.calls.len()
            {
                return Err(ProviderError::InvalidEnvelope);
            }
            let id = fragment
                .get("id")
                .and_then(Value::as_str)
                .ok_or(ProviderError::InvalidEnvelope)?;
            let name = fragment
                .get("function")
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            // The first fragment can carry arguments with the id — some
            // providers deliver a whole call in one chunk — so what arrived
            // with the id is the call's first argument bytes, not nothing.
            let arguments = fragment
                .get("function")
                .and_then(|function| function.get("arguments"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            self.calls.push(StreamCall {
                id: id.to_owned(),
                name: name.to_owned(),
                arguments: arguments.to_owned(),
                complete: false,
            });
            return Ok(());
        }
        let call = self
            .calls
            .get_mut(index)
            .ok_or(ProviderError::InvalidEnvelope)?;
        if call.complete {
            return Err(ProviderError::InvalidEnvelope);
        }
        if let Some(piece) = fragment
            .get("function")
            .and_then(|function| function.get("arguments"))
            .and_then(Value::as_str)
        {
            call.arguments.push_str(piece);
        } else {
            // A fragment with neither id nor arguments closes its call: the
            // provider's own end-of-stream marker for the call.
            call.complete = true;
        }
        Ok(())
    }

    /// Whether the provider named its finish reason.
    pub fn finished(&self) -> bool {
        self.finish.is_some()
    }

    /// The assembled response, valid only once the provider finished. The
    /// usage the stream never carried stays unknown for the reason it is
    /// unknown; it does not become a zero.
    pub fn assemble(
        self,
        request: &ProviderRequest,
        model: &symbiote_runtime_sdk::provider::ModelDescriptor,
    ) -> Result<ProviderResponse, ProviderError> {
        let finish = self.finish.ok_or(ProviderError::InvalidEnvelope)?;
        let mut tool_calls = Vec::new();
        for call in self.calls {
            let arguments: Value = serde_json::from_str(&call.arguments)
                .map_err(|_| ProviderError::InvalidEnvelope)?;
            tool_calls.push(ProviderToolCall {
                call_id: RequestId::new(&call.id).map_err(|_| ProviderError::InvalidEnvelope)?,
                name: call.name,
                arguments,
            });
        }
        let seen = tool_calls
            .iter()
            .map(|call| call.call_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        if seen.len() != tool_calls.len() {
            return Err(ProviderError::DuplicateToolCall);
        }
        let response = ProviderResponse {
            schema_version: PROVIDER_CONTRACT_VERSION,
            request_id: request.request_id.clone(),
            provider_id: request.provider_id.clone(),
            model_id: request.model_id.clone(),
            text: self.text,
            tool_calls,
            finish_reason: finish,
            usage: self.usage.unwrap_or(TokenUsage::Unknown {
                reason: UsageUnknownReason::NotReported,
            }),
        };
        response.validate(request, model)?;
        Ok(response)
    }
}
