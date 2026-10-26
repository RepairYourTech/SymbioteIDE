//! The protocol fixtures: a real local TCP endpoint speaking the provider's
//! shapes, and the adapter held against it — whole responses, streamed responses
//! with tool-call arguments fragmented across chunks, rate limits, redirects,
//! streams cut short, and the refusals that must never reach the network at all.
//!
//! No live provider is contacted; the "OpenAI-compatible label" here is the
//! in-process server's own dialect, which is exactly what makes these tests about
//! the protocol and not about a vendor.
use std::io::{Read, Write};
use std::time::Duration;

use std::collections::BTreeSet;
use symbiote_domain::{ModelId, ProviderConnectionId, RequestId};
use symbiote_native_agent::InferenceTransport;
use symbiote_provider_openai::{
    AllowedHosts, CatalogModel, CredentialSource, Endpoint, FixedCredential, OpenAiTransport,
    ProviderCatalog, StreamAssembler,
};
use symbiote_runtime_sdk::provider::{
    FinishReason, InputMessage, InputPart, MessageRole, ModelCapabilities, ModelDescriptor,
    ProviderError, ProviderRequest, ReasoningEffort, TokenUsage, ToolDefinition,
};

const HOST: &str = "127.0.0.1";

/// One scripted provider exchange: the server reads one request, answers with
/// the scripted bytes, and closes. The count of accepted connections is the
/// fixture's proof of what did and did not reach the provider.
struct Simulator {
    port: u16,
    connections: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Simulator {
    /// Serves `answer` for every connection: `answer` receives the request
    /// body it was sent and returns the raw HTTP response bytes.
    fn new(answer: impl Fn(Vec<u8>) -> Vec<u8> + Send + 'static) -> Self {
        let listener = std::net::TcpListener::bind((HOST, 0)).expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let connections = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let served = std::sync::Arc::clone(&connections);
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                served.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let mut stream = stream.expect("accept");
                let mut raw = Vec::new();
                let mut scratch = [0u8; 4096];
                // Read the head, then the body its Content-Length names.
                let head_end = loop {
                    if let Some(position) = find_head_end(&raw) {
                        break position;
                    }
                    match stream.read(&mut scratch) {
                        Ok(0) => break raw.len(),
                        Ok(read) => raw.extend_from_slice(&scratch[..read]),
                        Err(_) => break raw.len(),
                    }
                };
                let body_length = String::from_utf8_lossy(&raw[..head_end])
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                while raw.len() < head_end + 4 + body_length {
                    match stream.read(&mut scratch) {
                        Ok(0) => break,
                        Ok(read) => raw.extend_from_slice(&scratch[..read]),
                        Err(_) => break,
                    }
                }
                let body = raw[head_end + 4..].to_vec();
                let response = answer(body);
                let _ = stream.write_all(&response);
                let _ = stream.flush();
            }
        });
        Self {
            port,
            connections,
            handle: Some(handle),
        }
    }

    fn transport(&self, catalog: ProviderCatalog) -> OpenAiTransport {
        OpenAiTransport::new(
            Endpoint::new(HOST, self.port, "/v1/chat/completions").expect("a sane endpoint"),
            AllowedHosts::new([HOST]),
            Box::new(FixedCredential::new("the-connection-credential")),
            catalog,
            Duration::from_secs(5),
        )
        .expect("the endpoint is allowlisted")
    }
}

impl Drop for Simulator {
    fn drop(&mut self) {
        // Dropping the listener (owned by the thread) ends the loop; the
        // thread itself is left to finish its current connection.
        drop(self.handle.take());
    }
}

fn find_head_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http(body: &str) -> Vec<u8> {
    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}")
        .into_bytes()
}

fn descriptor(model: &str) -> ModelDescriptor {
    ModelDescriptor {
        schema_version: 1,
        id: ModelId::new(model).expect("a valid model id"),
        provider_id: ProviderConnectionId::new("conn-a").expect("a valid connection id"),
        context_window_tokens: 8192,
        max_output_tokens: 2048,
        capabilities: ModelCapabilities {
            reasoning_efforts: BTreeSet::from([
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High,
            ]),
            tools: true,
            images: false,
            streaming: true,
        },
    }
}

fn catalog(model: &str) -> ProviderCatalog {
    ProviderCatalog {
        catalog_version: 1,
        models: vec![CatalogModel {
            model_id: ModelId::new(model).expect("a valid model id"),
            context_window_tokens: 8192,
            max_output_tokens: 2048,
            supported_reasoning: BTreeSet::from([
                ReasoningEffort::Low,
                ReasoningEffort::Medium,
                ReasoningEffort::High,
            ]),
            tools: true,
        }],
    }
}

fn request(model: &str, streaming: bool) -> ProviderRequest {
    ProviderRequest {
        schema_version: 1,
        request_id: RequestId::new("req-a").expect("a valid request id"),
        provider_id: ProviderConnectionId::new("conn-a").expect("a valid connection id"),
        model_id: ModelId::new(model).expect("a valid model id"),
        messages: vec![InputMessage {
            role: MessageRole::User,
            content: vec![InputPart::Text {
                text: "do the work".into(),
            }],
        }],
        tools: vec![ToolDefinition {
            name: "shell".into(),
            description: "run a sandboxed command".into(),
            input_schema: serde_json::json!({"type": "object"}),
        }],
        input_token_budget: 128,
        max_output_tokens: 512,
        reasoning: Some(ReasoningEffort::Medium),
        streaming,
    }
}

fn whole_response_body() -> String {
    serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "the work is done",
                "tool_calls": [{
                    "id": "call-1",
                    "type": "function",
                    "function": {"name": "shell", "arguments": "{\"command\":\"echo hi\"}"}
                }]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {
            "prompt_tokens": 40,
            "completion_tokens": 12,
            "prompt_tokens_details": {"cached_tokens": 16},
            "completion_tokens_details": {"reasoning_tokens": 5}
        }
    })
    .to_string()
}

#[test]
fn a_whole_response_decodes_with_its_usage_semantics_preserved() {
    let simulator = Simulator::new(|_body| http(&whole_response_body()));
    let mut transport = simulator.transport(catalog("model-a"));
    let outcome = transport
        .request(&request("model-a", false), &descriptor("model-a"))
        .expect("the fixture completes");
    assert_eq!(outcome.text, "the work is done");
    assert_eq!(outcome.finish_reason, FinishReason::ToolCalls);
    assert_eq!(outcome.tool_calls.len(), 1);
    assert_eq!(outcome.tool_calls[0].name, "shell");
    assert_eq!(
        outcome.tool_calls[0].arguments,
        serde_json::json!({"command": "echo hi"})
    );
    // The provider's semantics survive: totals stand as reported, and the
    // cached subset (input) and reasoning subset (output) ride beside them
    // rather than being counted into the totals or dropped.
    assert_eq!(
        outcome.usage,
        TokenUsage::Known {
            input_tokens: 40,
            output_tokens: 12,
            cached_input_tokens: Some(16),
            reasoning_tokens: Some(5),
        }
    );
}

#[test]
fn a_streamed_response_assembles_fragmented_tool_calls_and_streamed_usage() {
    let chunks = [
        r#"data: {"choices":[{"delta":{"role":"assistant","content":null},"index":0}]}"#,
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call-9","type":"function","function":{"name":"shell","arguments":""}}]},"index":0}]}"#,
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"com"}}]},"index":0}]}"#,
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"mand\":\"echo hi\"}"}}]},"index":0}]}"#,
        r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls","index":0}]}"#,
        r#"data: {"choices":[],"usage":{"prompt_tokens":30,"completion_tokens":9}}"#,
        "data: [DONE]",
    ];
    let body = format!("{}\n\n", chunks.join("\n\n"));
    let simulator = Simulator::new(move |_body| http(&body));
    let mut transport = simulator.transport(catalog("model-a"));
    let outcome = transport
        .request(&request("model-a", true), &descriptor("model-a"))
        .expect("the fixture completes");
    assert_eq!(outcome.finish_reason, FinishReason::ToolCalls);
    // The argument fragments arrived in three pieces and are one call.
    assert_eq!(outcome.tool_calls.len(), 1);
    assert_eq!(
        outcome.tool_calls[0].arguments,
        serde_json::json!({"command": "echo hi"})
    );
    // The stream's usage chunk is the usage the response carries.
    assert_eq!(
        outcome.usage,
        TokenUsage::Known {
            input_tokens: 30,
            output_tokens: 9,
            cached_input_tokens: None,
            reasoning_tokens: None,
        }
    );
}

#[test]
fn a_rate_limit_is_a_fault_a_retry_may_answer() {
    let simulator = Simulator::new(|_body| {
        b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 7\r\nConnection: close\r\n\r\n".to_vec()
    });
    let mut transport = simulator.transport(catalog("model-a"));
    let error = transport
        .request(&request("model-a", false), &descriptor("model-a"))
        .expect_err("a rate limit is not an answer");
    assert!(matches!(error, ProviderError::Unavailable));
}

#[test]
fn a_redirect_is_refused_by_policy() {
    let simulator = Simulator::new(|_body| {
        b"HTTP/1.1 302 Found\r\nLocation: http://evil.test/v1/chat/completions\r\nConnection: close\r\n\r\n"
            .to_vec()
    });
    let mut transport = simulator.transport(catalog("model-a"));
    let error = transport
        .request(&request("model-a", false), &descriptor("model-a"))
        .expect_err("a redirect is not an answer");
    // Refused by policy, not followed: the credential was written to exactly
    // one origin and evil.test received nothing at all.
    assert!(matches!(error, ProviderError::LocalPolicyRequired));
    assert_eq!(
        simulator
            .connections
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[test]
fn a_stream_cut_before_its_finish_is_never_a_completion() {
    let simulator = Simulator::new(|_body| {
        http(r#"data: {"choices":[{"delta":{"content":"part of a"},"index":0}]}"#)
    });
    let mut transport = simulator.transport(catalog("model-a"));
    let error = transport
        .request(&request("model-a", true), &descriptor("model-a"))
        .expect_err("an interrupted generation is not an answer");
    assert!(matches!(error, ProviderError::Unavailable));
}

#[test]
fn a_request_the_catalog_refuses_never_reaches_the_provider() {
    let simulator = Simulator::new(|_body| http(&whole_response_body()));
    let mut transport = simulator.transport(catalog("model-a"));

    // A model the catalog does not hold.
    let unknown = transport
        .request(&request("model-z", false), &descriptor("model-a"))
        .expect_err("an unknown model is refused");
    assert!(matches!(unknown, ProviderError::ModelMismatch));

    // A reasoning effort the entry does not support (the request asks for the
    // contract's highest effort; the entry names Low..High only).
    let mut effort = request("model-a", false);
    effort.reasoning = Some(ReasoningEffort::Ultra);
    let effort_refused = transport
        .request(&effort, &descriptor("model-a"))
        .expect_err("an unsupported effort is refused, not coerced");
    assert!(matches!(
        effort_refused,
        ProviderError::UnsupportedCapability
    ));

    // An output budget past the entry's bound.
    let mut budget = request("model-a", false);
    budget.max_output_tokens = 4096;
    let budget_refused = transport
        .request(&budget, &descriptor("model-a"))
        .expect_err("an over-budget output is refused");
    assert!(matches!(budget_refused, ProviderError::OutputLimit));

    // An input estimate past the context window.
    let mut context = request("model-a", false);
    context.input_token_budget = 100_000;
    let context_refused = transport
        .request(&context, &descriptor("model-a"))
        .expect_err("an over-budget context is refused");
    assert!(matches!(context_refused, ProviderError::ContextLimit));

    // Every refusal happened before the first byte: the provider saw nothing.
    assert_eq!(
        simulator
            .connections
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[test]
fn refused_credentials_and_request_faults_classify_by_name() {
    let simulator =
        Simulator::new(|_body| b"HTTP/1.1 401 Unauthorized\r\nConnection: close\r\n\r\n".to_vec());
    let mut transport = simulator.transport(catalog("model-a"));
    let error = transport
        .request(&request("model-a", false), &descriptor("model-a"))
        .expect_err("refused credentials are not an answer");
    assert!(matches!(error, ProviderError::CredentialRequired));
}

#[test]
fn a_call_id_that_is_not_an_id_refuses_the_envelope() {
    // The provider's call id must be a `RequestId`: the neutral vocabulary
    // bounds what an id is, and an unbounded provider id is a shape the
    // contract does not carry.
    let mut assembler = StreamAssembler::default();
    assembler
        .feed(&serde_json::json!({
            "choices": [{"delta": {"tool_calls": [{"index": 0, "id": "not a legal id!", "type": "function", "function": {"name": "shell", "arguments": "{}"}}]}, "index": 0}]
        }))
        .expect("the fragment itself is fed");
    assembler
        .feed(&serde_json::json!({
            "choices": [{"delta": {}, "finish_reason": "tool_calls", "index": 0}]
        }))
        .expect("the finish is fed");
    let request = request("model-a", true);
    let model = descriptor("model-a");
    let error = assembler
        .assemble(&request, &model)
        .expect_err("an id outside the vocabulary is refused");
    assert!(matches!(error, ProviderError::InvalidEnvelope));
}

#[test]
fn a_credential_source_failure_refuses_before_the_wire() {
    struct Refused;
    impl CredentialSource for Refused {
        fn credential(&self) -> Result<String, ProviderError> {
            Err(ProviderError::CredentialRequired)
        }
    }
    let simulator = Simulator::new(|_body| http(&whole_response_body()));
    let mut transport = OpenAiTransport::new(
        Endpoint::new(HOST, simulator.port, "/v1/chat/completions").expect("a sane endpoint"),
        AllowedHosts::new([HOST]),
        Box::new(Refused),
        catalog("model-a"),
        Duration::from_secs(5),
    )
    .expect("the endpoint is allowlisted");
    let error = transport
        .request(&request("model-a", false), &descriptor("model-a"))
        .expect_err("no credential, no request");
    assert!(matches!(error, ProviderError::CredentialRequired));
    assert_eq!(
        simulator
            .connections
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

/// The house census reads this document against the bounds the crate holds;
/// the case that reads it must exist and hold it.
#[test]
fn the_contract_document_names_the_bounds_this_adapter_enforces() {
    let document = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/contracts/provider-openai.md"
    ))
    .expect("the contract document exists");
    assert!(
        document.contains("256 KiB"),
        "the document states the envelope bound this adapter answers by"
    );
    assert!(
        document.contains("Redirects are refused"),
        "the document states the redirect rule this transport holds"
    );
    assert!(
        document.contains("catalog version 1"),
        "the document states the catalog contract version this reader speaks"
    );
}
