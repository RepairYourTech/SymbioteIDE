//! Inference contracts only: no agent loop, credentials, network, or Host writes.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
};
use symbiote_domain::*;

pub const PROVIDER_CONTRACT_VERSION: u32 = 1;
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
pub const MAX_MESSAGES: usize = 128;
pub const MAX_TOOLS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderError {
    UnsupportedVersion,
    InvalidDescriptor,
    InvalidEnvelope,
    EnvelopeTooLarge,
    ModelMismatch,
    UnsupportedCapability,
    ContextLimit,
    InputLimit,
    OutputLimit,
    BindingMismatch,
    CredentialRequired,
    UnexpectedCredential,
    ExpiredEntitlement,
    UnsupportedAuthenticationBilling,
    LocalPolicyRequired,
    InvalidUsage,
    UnknownTool,
    DuplicateToolCall,
    Unavailable,
}
impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ProviderError {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
    Ultra,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelCapabilities {
    pub reasoning_efforts: BTreeSet<ReasoningEffort>,
    pub tools: bool,
    pub images: bool,
    pub streaming: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelDescriptor {
    pub schema_version: u32,
    pub id: ModelId,
    pub provider_id: ProviderConnectionId,
    pub context_window_tokens: u64,
    pub max_output_tokens: u64,
    pub capabilities: ModelCapabilities,
}
impl ModelDescriptor {
    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.schema_version != PROVIDER_CONTRACT_VERSION {
            return Err(ProviderError::UnsupportedVersion);
        }
        if self.context_window_tokens == 0
            || self.max_output_tokens == 0
            || self.max_output_tokens > self.context_window_tokens
        {
            return Err(ProviderError::InvalidDescriptor);
        }
        Ok(())
    }
}

/// Policy supplied by an authenticated Host after resolving endpoint locality.
/// Not a wire grant and not proof that an arbitrary URL is actually local.
#[derive(Clone, Debug, Default)]
pub struct ProviderPolicy {
    pub verified_local_endpoints: BTreeMap<ProviderConnectionId, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderBinding {
    schema_version: u32,
    runtime_kind: RuntimeKind,
    runtime_profile_id: RuntimeProfileId,
    adapter_id: InferenceProviderAdapterId,
    provider_id: ProviderConnectionId,
    endpoint_reference: String,
    model_id: ModelId,
    credential_reference: Option<CredentialReferenceId>,
    entitlement_id: BillingEntitlementId,
    entitlement_expires_at: Timestamp,
}
impl ProviderBinding {
    pub fn runtime_kind(&self) -> RuntimeKind {
        self.runtime_kind
    }
    pub fn adapter_id(&self) -> &InferenceProviderAdapterId {
        &self.adapter_id
    }
    pub fn provider_id(&self) -> &ProviderConnectionId {
        &self.provider_id
    }
    pub fn endpoint_reference(&self) -> &str {
        &self.endpoint_reference
    }
    pub fn model_id(&self) -> &ModelId {
        &self.model_id
    }
    pub fn credential_reference(&self) -> Option<&CredentialReferenceId> {
        self.credential_reference.as_ref()
    }
    pub fn validate_at(&self, now: Timestamp) -> Result<(), ProviderError> {
        if self.entitlement_expires_at <= now {
            return Err(ProviderError::ExpiredEntitlement);
        }
        Ok(())
    }
}

/// The registration-facing half of [`validate_binding`]: everything a durable
/// provider registry can prove about a profile's binding on its own — model
/// validity, profile/connection/entitlement/model identity consistency, an
/// explicit endpoint reference, entitlement expiry and the native/harness
/// authentication-billing rejection. Credential *material* and verified local
/// endpoint policy are deliberately outside it: those are execution-time Host
/// concerns that [the full binding check](validate_binding) adds.
///
/// Registry consumers (dispatch preparation) validate stored records through
/// this same function, so a stored binding is checked by one set of rules
/// rather than a parallel copy that could drift from the binding contract.
pub fn validate_registration(
    profile: &RuntimeProfile,
    connection: &ProviderConnection,
    entitlement: &BillingEntitlement,
    model: &ModelDescriptor,
    now: Timestamp,
) -> Result<(), ProviderError> {
    model.validate()?;
    if profile.provider != connection.id
        || entitlement.provider != connection.id
        || entitlement.id != profile.billing_entitlement
        || model.provider_id != connection.id
        || model.id != profile.model
        || connection.endpoint_reference.trim().is_empty()
    {
        return Err(ProviderError::BindingMismatch);
    }
    if entitlement.expires_at <= now {
        return Err(ProviderError::ExpiredEntitlement);
    }
    // The supported mapping table is a registry fact too: an authentication
    // kind is only meaningful against the billing kind it was registered
    // with, and harness-managed subscription is an external-runtime mapping
    // only. An unsupported pair fails rather than switching billing methods.
    let supported = match (&connection.authentication, &entitlement.kind) {
        (AuthenticationKind::ApiCredential, BillingKind::MeteredApi) => true,
        (AuthenticationKind::HarnessManaged, BillingKind::HarnessSubscription) => {
            profile.runtime == RuntimeKind::ExternalHarness
        }
        (AuthenticationKind::LocalUnauthenticated, BillingKind::Local) => true,
        _ => false,
    };
    if !supported {
        return Err(ProviderError::UnsupportedAuthenticationBilling);
    }
    Ok(())
}

/// All inputs are canonical references, not secret values. Evidence authenticity,
/// endpoint locality and the actual runtime loop owner are Host responsibilities.
pub fn validate_binding(
    profile: &RuntimeProfile,
    connection: &ProviderConnection,
    credential: Option<&CredentialReference>,
    entitlement: &BillingEntitlement,
    model: &ModelDescriptor,
    policy: &ProviderPolicy,
    now: Timestamp,
) -> Result<ProviderBinding, ProviderError> {
    validate_registration(profile, connection, entitlement, model, now)?;
    // The registration check above already refused every unsupported
    // authentication/billing pair, so these arms cover the whole table.
    let credential_reference = match (&connection.authentication, &entitlement.kind) {
        (AuthenticationKind::ApiCredential, BillingKind::MeteredApi) => {
            let credential = credential.ok_or(ProviderError::CredentialRequired)?;
            if credential.id != profile.credential || credential.vault_key.trim().is_empty() {
                return Err(ProviderError::BindingMismatch);
            }
            Some(credential.id.clone())
        }
        (AuthenticationKind::HarnessManaged, BillingKind::HarnessSubscription) => {
            if credential.is_some() {
                return Err(ProviderError::UnexpectedCredential);
            }
            None
        }
        (AuthenticationKind::LocalUnauthenticated, BillingKind::Local) => {
            if credential.is_some() {
                return Err(ProviderError::UnexpectedCredential);
            }
            if policy.verified_local_endpoints.get(&connection.id)
                != Some(&connection.endpoint_reference)
            {
                return Err(ProviderError::LocalPolicyRequired);
            }
            None
        }
        _ => return Err(ProviderError::UnsupportedAuthenticationBilling),
    };
    Ok(ProviderBinding {
        schema_version: PROVIDER_CONTRACT_VERSION,
        runtime_kind: profile.runtime,
        runtime_profile_id: profile.id.clone(),
        adapter_id: connection.adapter.clone(),
        provider_id: connection.id.clone(),
        endpoint_reference: connection.endpoint_reference.clone(),
        model_id: model.id.clone(),
        credential_reference,
        entitlement_id: entitlement.id.clone(),
        entitlement_expires_at: entitlement.expires_at,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputPart {
    Text {
        text: String,
    },
    /// Host-resolved artifact reference, never an arbitrary URL or secret-bearing path.
    Image {
        artifact_id: ArtifactId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InputMessage {
    pub role: MessageRole,
    pub content: Vec<InputPart>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderRequest {
    pub schema_version: u32,
    pub request_id: RequestId,
    pub provider_id: ProviderConnectionId,
    pub model_id: ModelId,
    pub messages: Vec<InputMessage>,
    pub tools: Vec<ToolDefinition>,
    /// Upper-bound estimate supplied by the Context Broker; not provider-reported usage.
    pub input_token_budget: u64,
    pub max_output_tokens: u64,
    pub reasoning: Option<ReasoningEffort>,
    pub streaming: bool,
}

fn tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

fn validate_json(value: &Value) -> Result<(), ProviderError> {
    fn visit(value: &Value, depth: usize, remaining: &mut usize) -> Result<(), ProviderError> {
        if depth > 32 || *remaining == 0 {
            return Err(ProviderError::InvalidEnvelope);
        }
        *remaining -= 1;
        match value {
            Value::Array(items) => {
                for item in items {
                    visit(item, depth + 1, remaining)?;
                }
            }
            Value::Object(items) => {
                for item in items.values() {
                    visit(item, depth + 1, remaining)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(value, 0, &mut 4096)
}

fn bounded<T: Serialize>(value: &T) -> Result<(), ProviderError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_ENVELOPE_BYTES - self.0 {
                return Err(std::io::Error::other("provider envelope bound"));
            }
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(&mut Counter(0), value).map_err(|_| ProviderError::EnvelopeTooLarge)
}

impl ProviderRequest {
    pub fn parse(bytes: &[u8], model: &ModelDescriptor) -> Result<Self, ProviderError> {
        if bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(ProviderError::EnvelopeTooLarge);
        }
        let request: Self =
            serde_json::from_slice(bytes).map_err(|_| ProviderError::InvalidEnvelope)?;
        request.validate(model)?;
        Ok(request)
    }
    pub fn validate(&self, model: &ModelDescriptor) -> Result<(), ProviderError> {
        model.validate()?;
        if self.schema_version != PROVIDER_CONTRACT_VERSION {
            return Err(ProviderError::UnsupportedVersion);
        }
        if self.provider_id != model.provider_id || self.model_id != model.id {
            return Err(ProviderError::ModelMismatch);
        }
        if self.messages.is_empty()
            || self.messages.len() > MAX_MESSAGES
            || self.tools.len() > MAX_TOOLS
        {
            return Err(ProviderError::InvalidEnvelope);
        }
        if self.input_token_budget == 0
            || self
                .input_token_budget
                .checked_add(self.max_output_tokens)
                .is_none_or(|sum| sum > model.context_window_tokens)
        {
            return Err(ProviderError::ContextLimit);
        }
        if self.max_output_tokens == 0 || self.max_output_tokens > model.max_output_tokens {
            return Err(ProviderError::OutputLimit);
        }
        if (self.streaming && !model.capabilities.streaming)
            || (!self.tools.is_empty() && !model.capabilities.tools)
            || self
                .reasoning
                .as_ref()
                .is_some_and(|effort| !model.capabilities.reasoning_efforts.contains(effort))
        {
            return Err(ProviderError::UnsupportedCapability);
        }
        for message in &self.messages {
            if message.content.is_empty() || message.content.len() > 32 {
                return Err(ProviderError::InvalidEnvelope);
            }
            for part in &message.content {
                match part {
                    InputPart::Text { text } if text.is_empty() => {
                        return Err(ProviderError::InvalidEnvelope);
                    }
                    InputPart::Image { .. } if !model.capabilities.images => {
                        return Err(ProviderError::UnsupportedCapability);
                    }
                    _ => {}
                }
            }
        }
        let mut names = BTreeSet::new();
        for tool in &self.tools {
            if !tool_name(&tool.name) || !tool.input_schema.is_object() || !names.insert(&tool.name)
            {
                return Err(ProviderError::InvalidEnvelope);
            }
            validate_json(&tool.input_schema)?;
        }
        bounded(self)
    }
    pub fn validate_binding(
        &self,
        binding: &ProviderBinding,
        now: Timestamp,
    ) -> Result<(), ProviderError> {
        binding.validate_at(now)?;
        if self.provider_id != binding.provider_id || self.model_id != binding.model_id {
            return Err(ProviderError::BindingMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageUnknownReason {
    NotReported,
    Interrupted,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TokenUsage {
    Unknown {
        reason: UsageUnknownReason,
    },
    Known {
        input_tokens: u64,
        output_tokens: u64,
        cached_input_tokens: Option<u64>,
        reasoning_tokens: Option<u64>,
    },
}
impl TokenUsage {
    /// Unknown totals remain None, never zero. Reasoning is a subset of output;
    /// cached tokens are a subset of input, so neither is counted twice.
    pub fn total(&self) -> Result<Option<u64>, ProviderError> {
        match self {
            Self::Unknown { .. } => Ok(None),
            Self::Known {
                input_tokens,
                output_tokens,
                cached_input_tokens,
                reasoning_tokens,
            } => {
                if cached_input_tokens.is_some_and(|n| n > *input_tokens)
                    || reasoning_tokens.is_some_and(|n| n > *output_tokens)
                {
                    return Err(ProviderError::InvalidUsage);
                }
                input_tokens
                    .checked_add(*output_tokens)
                    .map(Some)
                    .ok_or(ProviderError::InvalidUsage)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderToolCall {
    /// Canonical request correlation, not an authorization token or raw provider ID.
    pub call_id: RequestId,
    pub name: String,
    pub arguments: Value,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderResponse {
    pub schema_version: u32,
    pub request_id: RequestId,
    pub provider_id: ProviderConnectionId,
    pub model_id: ModelId,
    pub text: String,
    pub tool_calls: Vec<ProviderToolCall>,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
}
impl ProviderResponse {
    pub fn parse(
        bytes: &[u8],
        request: &ProviderRequest,
        model: &ModelDescriptor,
    ) -> Result<Self, ProviderError> {
        if bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(ProviderError::EnvelopeTooLarge);
        }
        let response: Self =
            serde_json::from_slice(bytes).map_err(|_| ProviderError::InvalidEnvelope)?;
        response.validate(request, model)?;
        Ok(response)
    }
    pub fn validate(
        &self,
        request: &ProviderRequest,
        model: &ModelDescriptor,
    ) -> Result<(), ProviderError> {
        request.validate(model)?;
        if self.schema_version != PROVIDER_CONTRACT_VERSION {
            return Err(ProviderError::UnsupportedVersion);
        }
        if self.request_id != request.request_id
            || self.provider_id != request.provider_id
            || self.model_id != request.model_id
        {
            return Err(ProviderError::ModelMismatch);
        }
        if self.tool_calls.len() > MAX_TOOLS {
            return Err(ProviderError::InvalidEnvelope);
        }
        if (self.finish_reason == FinishReason::ToolCalls) != !self.tool_calls.is_empty() {
            return Err(ProviderError::InvalidEnvelope);
        }
        if self.finish_reason == FinishReason::Stop && self.text.is_empty() {
            return Err(ProviderError::InvalidEnvelope);
        }
        let mut ids = BTreeSet::new();
        for call in &self.tool_calls {
            if !ids.insert(&call.call_id) {
                return Err(ProviderError::DuplicateToolCall);
            }
            if !request.tools.iter().any(|tool| tool.name == call.name) {
                return Err(ProviderError::UnknownTool);
            }
            if !call.arguments.is_object() {
                return Err(ProviderError::InvalidEnvelope);
            }
            validate_json(&call.arguments)?;
        }
        if self
            .usage
            .total()?
            .is_some_and(|total| total > model.context_window_tokens)
        {
            return Err(ProviderError::ContextLimit);
        }
        if let TokenUsage::Known {
            input_tokens,
            output_tokens,
            ..
        } = self.usage
        {
            if input_tokens > request.input_token_budget {
                return Err(ProviderError::InputLimit);
            }
            if output_tokens > request.max_output_tokens {
                return Err(ProviderError::OutputLimit);
            }
        }
        bounded(self)
    }
}

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderResponse, ProviderError>> + Send + 'a>>;

/// A model turn only, distinct from the crate's AgentRuntimeAdapter (loop owner).
/// Implementations must validate request/model/binding, recheck expiry when used,
/// and validate the final response. No implementation is supplied in this slice.
pub trait InferenceProviderAdapter: Send + Sync {
    fn adapter_id(&self) -> &InferenceProviderAdapterId;
    fn infer<'a>(
        &'a self,
        request: &'a ProviderRequest,
        binding: &'a ProviderBinding,
    ) -> ProviderFuture<'a>;
}
