//! Mock transport tests establish discovery contracts, not a live integration.
use serde_json::{Value, json};
use std::{collections::VecDeque, time::Duration};
use symbiote_runtime_discovery::codex::*;

struct Fixture {
    replies: VecDeque<Value>,
    sent: Vec<Value>,
    delay: Duration,
}
impl DiscoveryTransport for Fixture {
    fn send(&mut self, value: &Value, _: Duration) -> Result<(), CodexDiscoveryError> {
        self.sent.push(value.clone());
        Ok(())
    }
    fn recv(&mut self, _: Duration) -> Result<Value, CodexDiscoveryError> {
        std::thread::sleep(self.delay);
        self.replies
            .pop_front()
            .ok_or(CodexDiscoveryError::Transport)
    }
}
fn model(id: &str) -> Value {
    json!({"id":id,"model":"gpt-5.4","isDefault":true,"hidden":false,"defaultReasoningEffort":"medium","supportedReasoningEfforts":[{"reasoningEffort":"medium","description":"ignored"}],"displayName":"ignored","description":"ignored"})
}
fn fixture() -> Fixture {
    Fixture {
        sent: vec![],
        delay: Duration::ZERO,
        replies: VecDeque::from([
            json!({"id":1,"result":{"userAgent":"symbiote/0.118.0 (Linux)"}}),
            json!({"id":2,"result":{"requiresOpenaiAuth":true,"account":null}}),
            json!({"id":3,"result":{"data":[model("picker.one")],"nextCursor":null}}),
        ]),
    }
}

#[test]
fn handshake_is_versionless_read_only_and_fields_not_reported_remain_unknown() {
    let mut fixture = fixture();
    let report = discover(&mut fixture, Duration::from_secs(1)).unwrap();
    assert_eq!(report.authentication, CodexAuthentication::Required);
    assert_eq!(report.server_version, "0.118.0");
    assert_eq!(fixture.sent[0]["params"]["clientInfo"]["version"], "0.1.0");
    assert_eq!(report.models[0].provider_model, "gpt-5.4");
    assert_eq!(report.models[0].context_tokens, None);
    assert_eq!(report.models[0].input_modalities, None);
    let methods: Vec<_> = fixture
        .sent
        .iter()
        .map(|v| v["method"].as_str().unwrap())
        .collect();
    assert_eq!(
        methods,
        ["initialize", "initialized", "account/read", "model/list"]
    );
    assert!(fixture.sent.iter().all(|v| v.get("jsonrpc").is_none()));
    assert_eq!(fixture.sent[2]["params"]["refreshToken"], false);
    assert_eq!(fixture.sent[3]["params"]["limit"], 32);
}

#[test]
fn account_metadata_and_error_contents_are_discarded() {
    let mut fixture = fixture();
    fixture.replies[1] = json!({"id":2,"result":{"requiresOpenaiAuth":true,"account":{"type":"chatgpt","email":"private@example.test","planType":"pro","token":"private-token"}}});
    let report = discover(&mut fixture, Duration::from_secs(1)).unwrap();
    assert_eq!(report.authentication, CodexAuthentication::ChatGptReported);
    let output = format!("{report:?}{}", serde_json::to_string(&report).unwrap());
    assert!(!output.contains("private"));
    assert!(!output.contains("pro\""));
    let mut fixture = crate::fixture();
    fixture.replies[1] = json!({"id":2,"error":{"code":-1,"message":"private-token","data":{"email":"private@example.test"}}});
    let error = discover(&mut fixture, Duration::from_secs(1)).unwrap_err();
    assert_eq!(error, CodexDiscoveryError::RpcFailure);
    assert!(!format!("{error:?} {error}").contains("private"));
}

#[test]
fn exact_server_version_and_correlation_are_required() {
    for frame in [
        json!({"id":1,"result":{"userAgent":"symbiote/0.118.1 (Linux)"}}),
        json!({"id":1,"result":{"userAgent":"unrelated symbiote/0.118.0"}}),
        json!({"id":2,"result":{}}),
        json!({"id":1,"result":{},"error":{}}),
        json!({"id":1,"method":"initialize","params":{}}),
    ] {
        let mut fixture = fixture();
        fixture.replies[0] = frame;
        assert!(discover(&mut fixture, Duration::from_secs(1)).is_err());
    }
    let mut fixture = fixture();
    fixture.replies[1] = fixture.replies[0].clone();
    assert_eq!(
        discover(&mut fixture, Duration::from_secs(1)),
        Err(CodexDiscoveryError::UnexpectedResponse)
    );
}

#[test]
fn pagination_is_bounded_correlated_and_deterministic() {
    let mut fixture = fixture();
    fixture.replies[2]["result"]["nextCursor"] = json!("page-two");
    fixture
        .replies
        .push_back(json!({"id":4,"result":{"data":[model("a-picker")],"nextCursor":null}}));
    let report = discover(&mut fixture, Duration::from_secs(1)).unwrap();
    assert_eq!(report.models[0].listing_id, "a-picker");
    assert_eq!(fixture.sent[4]["params"]["cursor"], "page-two");
    let mut fixture = crate::fixture();
    fixture.replies[2]["result"]["nextCursor"] = json!("same");
    fixture
        .replies
        .push_back(json!({"id":4,"result":{"data":[],"nextCursor":"same"}}));
    assert_eq!(
        discover(&mut fixture, Duration::from_secs(1)),
        Err(CodexDiscoveryError::RepeatedCursor)
    );
    let mut fixture = crate::fixture();
    fixture.replies[2]["result"]["data"] = json!([model("same"), model("same")]);
    assert_eq!(
        discover(&mut fixture, Duration::from_secs(1)),
        Err(CodexDiscoveryError::DuplicateModel)
    );
}

#[test]
fn notification_flood_and_absolute_deadline_fail_without_reset() {
    let mut fixture = fixture();
    for _ in 0..33 {
        fixture
            .replies
            .push_front(json!({"method":"account/updated","params":{"ignored":"private"}}));
    }
    assert_eq!(
        discover(&mut fixture, Duration::from_secs(1)),
        Err(CodexDiscoveryError::LimitExceeded)
    );
    let mut fixture = crate::fixture();
    fixture.delay = Duration::from_millis(10);
    assert_eq!(
        discover(&mut fixture, Duration::from_millis(1)),
        Err(CodexDiscoveryError::Deadline)
    );
    assert_eq!(fixture.sent.len(), 1);
}

#[test]
fn advertised_capabilities_are_never_guessed() {
    let mut fixture = fixture();
    fixture.replies[2]["result"]["data"][0]["inputModalities"] = json!(["text"]);
    fixture.replies[2]["result"]["data"][0]["contextWindow"] = json!(99999);
    let report = discover(&mut fixture, Duration::from_secs(1)).unwrap();
    assert_eq!(report.models[0].context_tokens, None);
    assert_eq!(report.models[0].input_modalities.as_ref().unwrap().len(), 1);
    let mut fixture = crate::fixture();
    fixture.replies[2]["result"]["data"][0]["defaultReasoningEffort"] = json!("ultra");
    assert_eq!(
        discover(&mut fixture, Duration::from_secs(1)),
        Err(CodexDiscoveryError::MalformedFrame)
    );
}

#[test]
fn optional_account_and_cursor_follow_pinned_schema() {
    // 0.118.0 GetAccountResponse requires only requiresOpenaiAuth;
    // ModelListResponse requires only data. Optional account/cursor may be absent.
    let mut fixture = fixture();
    fixture.replies[1]["result"]
        .as_object_mut()
        .unwrap()
        .remove("account");
    fixture.replies[2]["result"]
        .as_object_mut()
        .unwrap()
        .remove("nextCursor");
    let report = discover(&mut fixture, Duration::from_secs(1)).unwrap();
    assert_eq!(report.authentication, CodexAuthentication::Required);
    assert_eq!(report.models.len(), 1);
}
