//! Read-only Codex App Server discovery. Results are advertisements, never proof
//! of credential validity, entitlement, model usability or execution authority.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};

pub const CODEX_VERSION: &str = "0.118.0";
const MAX_MODELS: usize = 256;
const PAGE_SIZE: usize = 32;
const MAX_PAGES: usize = 8;
const MAX_NOTIFICATIONS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexDiscoveryError {
    Transport,
    Deadline,
    MalformedFrame,
    UnexpectedResponse,
    RpcFailure,
    UnsupportedVersion,
    LimitExceeded,
    DuplicateModel,
    RepeatedCursor,
}
impl std::fmt::Display for CodexDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Codex discovery failed")
    }
}
impl std::error::Error for CodexDiscoveryError {}

/// Implementations must bound framing and reject duplicate JSON object keys
/// before producing Value (the shared JsonlTransport does both). No raw transport
/// error strings may be retained. Every call must respect its supplied timeout.
pub trait DiscoveryTransport {
    fn send(&mut self, message: &Value, timeout: Duration) -> Result<(), CodexDiscoveryError>;
    fn recv(&mut self, timeout: Duration) -> Result<Value, CodexDiscoveryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexAuthentication {
    Required,
    NotRequired,
    ApiKeyReported,
    ChatGptReported,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexInputModality {
    Text,
    Image,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexModel {
    /// Server picker identity, distinct from model name and canonical domain ID.
    pub listing_id: String,
    pub provider_model: String,
    pub is_default: bool,
    pub hidden: bool,
    pub supported_reasoning: BTreeSet<CodexReasoningEffort>,
    pub default_reasoning: CodexReasoningEffort,
    /// Absent fields remain unknown, even if upstream schema declares defaults.
    pub input_modalities: Option<BTreeSet<CodexInputModality>>,
    pub context_tokens: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexProbeReport {
    pub server_version: String,
    pub authentication: CodexAuthentication,
    pub models: Vec<CodexModel>,
}

fn remaining(deadline: Instant) -> Result<Duration, CodexDiscoveryError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(CodexDiscoveryError::Deadline)
}
fn bounded_frame(value: &Value) -> Result<(), CodexDiscoveryError> {
    let mut stack = vec![(value, 0)];
    let mut nodes = 0;
    let mut bytes = 0usize;
    while let Some((value, depth)) = stack.pop() {
        nodes += 1;
        if nodes > 65_536 || depth > 64 {
            return Err(CodexDiscoveryError::LimitExceeded);
        }
        match value {
            Value::String(s) => bytes = bytes.saturating_add(s.len()),
            Value::Array(a) => {
                if a.len() > 65_536usize.saturating_sub(nodes + stack.len()) {
                    return Err(CodexDiscoveryError::LimitExceeded);
                }
                stack.extend(a.iter().map(|v| (v, depth + 1)));
            }
            Value::Object(o) => {
                if o.len() > 65_536usize.saturating_sub(nodes + stack.len()) {
                    return Err(CodexDiscoveryError::LimitExceeded);
                }
                for (k, v) in o {
                    bytes = bytes.saturating_add(k.len());
                    stack.push((v, depth + 1));
                }
            }
            _ => {}
        }
        if bytes > 1_048_576 {
            return Err(CodexDiscoveryError::LimitExceeded);
        }
    }
    Ok(())
}
fn response(
    transport: &mut impl DiscoveryTransport,
    id: u64,
    deadline: Instant,
    notifications: &mut usize,
) -> Result<Value, CodexDiscoveryError> {
    loop {
        let frame = transport.recv(remaining(deadline)?)?;
        remaining(deadline)?;
        bounded_frame(&frame)?;
        let object = frame
            .as_object()
            .ok_or(CodexDiscoveryError::MalformedFrame)?;
        if object.contains_key("method") {
            if object
                .keys()
                .any(|k| !matches!(k.as_str(), "method" | "params"))
                || !object["method"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() <= 256)
            {
                return Err(CodexDiscoveryError::MalformedFrame);
            }
            *notifications += 1;
            if *notifications > MAX_NOTIFICATIONS {
                return Err(CodexDiscoveryError::LimitExceeded);
            }
            continue;
        }
        if object
            .keys()
            .any(|k| !matches!(k.as_str(), "id" | "result" | "error"))
            || object.get("id").and_then(Value::as_u64) != Some(id)
        {
            return Err(CodexDiscoveryError::UnexpectedResponse);
        }
        match (object.get("result"), object.get("error")) {
            (Some(result), None) if result.is_object() => return Ok(result.clone()),
            (None, Some(error)) if error.is_object() => {
                return Err(CodexDiscoveryError::RpcFailure);
            }
            _ => return Err(CodexDiscoveryError::MalformedFrame),
        }
    }
}
fn call(
    transport: &mut impl DiscoveryTransport,
    id: u64,
    method: &str,
    params: Value,
    deadline: Instant,
    notifications: &mut usize,
) -> Result<Value, CodexDiscoveryError> {
    transport.send(
        &json!({"id":id,"method":method,"params":params}),
        remaining(deadline)?,
    )?;
    response(transport, id, deadline, notifications)
}
fn identifier(value: Option<&Value>) -> Result<String, CodexDiscoveryError> {
    let s = value
        .and_then(Value::as_str)
        .ok_or(CodexDiscoveryError::MalformedFrame)?;
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:/-".contains(&b))
    {
        return Err(CodexDiscoveryError::MalformedFrame);
    }
    Ok(s.into())
}
fn effort(value: Option<&Value>) -> Result<CodexReasoningEffort, CodexDiscoveryError> {
    match value.and_then(Value::as_str) {
        Some("none") => Ok(CodexReasoningEffort::None),
        Some("minimal") => Ok(CodexReasoningEffort::Minimal),
        Some("low") => Ok(CodexReasoningEffort::Low),
        Some("medium") => Ok(CodexReasoningEffort::Medium),
        Some("high") => Ok(CodexReasoningEffort::High),
        Some("xhigh") => Ok(CodexReasoningEffort::Xhigh),
        _ => Err(CodexDiscoveryError::MalformedFrame),
    }
}
fn model(value: &Value) -> Result<CodexModel, CodexDiscoveryError> {
    let options = value
        .get("supportedReasoningEfforts")
        .and_then(Value::as_array)
        .ok_or(CodexDiscoveryError::MalformedFrame)?;
    if options.len() > 6 {
        return Err(CodexDiscoveryError::LimitExceeded);
    }
    let mut supported_reasoning = BTreeSet::new();
    for option in options {
        if !supported_reasoning.insert(effort(option.get("reasoningEffort"))?) {
            return Err(CodexDiscoveryError::MalformedFrame);
        }
    }
    let default_reasoning = effort(value.get("defaultReasoningEffort"))?;
    if !supported_reasoning.contains(&default_reasoning) {
        return Err(CodexDiscoveryError::MalformedFrame);
    }
    let input_modalities = match value.get("inputModalities") {
        None => None,
        Some(Value::Array(options)) if options.len() <= 2 => {
            let mut set = BTreeSet::new();
            for option in options {
                let modality = match option.as_str() {
                    Some("text") => CodexInputModality::Text,
                    Some("image") => CodexInputModality::Image,
                    _ => return Err(CodexDiscoveryError::MalformedFrame),
                };
                if !set.insert(modality) {
                    return Err(CodexDiscoveryError::MalformedFrame);
                }
            }
            Some(set)
        }
        _ => return Err(CodexDiscoveryError::MalformedFrame),
    };
    Ok(CodexModel {
        listing_id: identifier(value.get("id"))?,
        provider_model: identifier(value.get("model"))?,
        is_default: value
            .get("isDefault")
            .and_then(Value::as_bool)
            .ok_or(CodexDiscoveryError::MalformedFrame)?,
        hidden: value
            .get("hidden")
            .and_then(Value::as_bool)
            .ok_or(CodexDiscoveryError::MalformedFrame)?,
        supported_reasoning,
        default_reasoning,
        input_modalities,
        context_tokens: None,
    })
}

/// Probe exactly initialize/initialized/account-read/model-list, never a login,
/// thread, turn, install, configuration write or credential refresh operation.
pub fn discover(
    transport: &mut impl DiscoveryTransport,
    timeout: Duration,
) -> Result<CodexProbeReport, CodexDiscoveryError> {
    if timeout.is_zero() || timeout > Duration::from_secs(30) {
        return Err(CodexDiscoveryError::Deadline);
    }
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(CodexDiscoveryError::Deadline)?;
    let mut notifications = 0;
    let initialized = call(
        transport,
        1,
        "initialize",
        json!({"clientInfo":{"name":"symbiote","version":"0.1.0","title":"Symbiote"}}),
        deadline,
        &mut notifications,
    )?;
    let user_agent = initialized
        .get("userAgent")
        .and_then(Value::as_str)
        .ok_or(CodexDiscoveryError::MalformedFrame)?;
    if user_agent.len() > 1024 || user_agent.split_whitespace().next() != Some("symbiote/0.118.0") {
        return Err(CodexDiscoveryError::UnsupportedVersion);
    }
    transport.send(
        &json!({"method":"initialized","params":{}}),
        remaining(deadline)?,
    )?;
    let account = call(
        transport,
        2,
        "account/read",
        json!({"refreshToken":false}),
        deadline,
        &mut notifications,
    )?;
    let required = account
        .get("requiresOpenaiAuth")
        .and_then(Value::as_bool)
        .ok_or(CodexDiscoveryError::MalformedFrame)?;
    let authentication = match account.get("account") {
        None | Some(Value::Null) => {
            if required {
                CodexAuthentication::Required
            } else {
                CodexAuthentication::NotRequired
            }
        }
        Some(account) => match account.get("type").and_then(Value::as_str) {
            Some("apiKey") => CodexAuthentication::ApiKeyReported,
            Some("chatgpt") => CodexAuthentication::ChatGptReported,
            Some(_) => CodexAuthentication::Unknown,
            None => return Err(CodexDiscoveryError::MalformedFrame),
        },
    };
    let mut models = Vec::new();
    let mut ids = BTreeSet::new();
    let mut cursors = BTreeSet::new();
    let mut cursor = None;
    for page in 0..MAX_PAGES {
        let result = call(
            transport,
            3 + page as u64,
            "model/list",
            json!({"limit":PAGE_SIZE,"includeHidden":false,"cursor":cursor}),
            deadline,
            &mut notifications,
        )?;
        let data = result
            .get("data")
            .and_then(Value::as_array)
            .ok_or(CodexDiscoveryError::MalformedFrame)?;
        if data.len() > PAGE_SIZE || models.len() + data.len() > MAX_MODELS {
            return Err(CodexDiscoveryError::LimitExceeded);
        }
        for item in data {
            let item = model(item)?;
            if !ids.insert(item.listing_id.clone()) {
                return Err(CodexDiscoveryError::DuplicateModel);
            }
            models.push(item);
        }
        match result.get("nextCursor") {
            None | Some(Value::Null) => {
                remaining(deadline)?;
                models.sort_by(|a, b| a.listing_id.cmp(&b.listing_id));
                return Ok(CodexProbeReport {
                    server_version: CODEX_VERSION.into(),
                    authentication,
                    models,
                });
            }
            Some(Value::String(next))
                if !next.is_empty()
                    && next.len() <= 1024
                    && !next.chars().any(char::is_control) =>
            {
                if !cursors.insert(next.clone()) {
                    return Err(CodexDiscoveryError::RepeatedCursor);
                }
                cursor = Some(next.clone());
            }
            _ => return Err(CodexDiscoveryError::MalformedFrame),
        }
    }
    Err(CodexDiscoveryError::LimitExceeded)
}
