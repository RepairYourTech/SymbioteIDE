use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use symbiote_domain::*;
use symbiote_runtime_sdk::provider::*;

fn fixtures() -> (
    RuntimeProfile,
    ProviderConnection,
    CredentialReference,
    BillingEntitlement,
    ModelDescriptor,
) {
    let provider_id = ProviderConnectionId::new("provider").unwrap();
    let credential_id = CredentialReferenceId::new("credential").unwrap();
    let entitlement_id = BillingEntitlementId::new("entitlement").unwrap();
    let model_id = ModelId::new("model").unwrap();
    (
        RuntimeProfile {
            id: RuntimeProfileId::new("profile").unwrap(),
            revision: Revision(1),
            runtime: RuntimeKind::NativeSymbiote,
            adapter: AgentRuntimeAdapterId::new("native-loop").unwrap(),
            installation: None,
            provider: provider_id.clone(),
            credential: credential_id.clone(),
            billing_entitlement: entitlement_id.clone(),
            model: model_id.clone(),
            eligible_hosts: BTreeSet::from([HostId::new("host").unwrap()]),
        },
        ProviderConnection {
            id: provider_id.clone(),
            adapter: InferenceProviderAdapterId::new("inference").unwrap(),
            endpoint_reference: "endpoint-main".into(),
            authentication: AuthenticationKind::ApiCredential,
        },
        CredentialReference {
            id: credential_id,
            vault_key: "vault-reference-only".into(),
        },
        BillingEntitlement {
            id: entitlement_id,
            provider: provider_id.clone(),
            kind: BillingKind::MeteredApi,
            verification_evidence: EvidenceId::new("entitlement-proof").unwrap(),
            expires_at: Timestamp(100),
        },
        ModelDescriptor {
            schema_version: 1,
            id: model_id,
            provider_id,
            context_window_tokens: 128,
            max_output_tokens: 64,
            capabilities: ModelCapabilities {
                reasoning_efforts: [ReasoningEffort::Low].into(),
                tools: true,
                images: true,
                streaming: true,
            },
        },
    )
}
fn request() -> ProviderRequest {
    ProviderRequest {
        schema_version: 1,
        request_id: RequestId::new("inference-request").unwrap(),
        provider_id: ProviderConnectionId::new("provider").unwrap(),
        model_id: ModelId::new("model").unwrap(),
        messages: vec![InputMessage {
            role: MessageRole::User,
            content: vec![InputPart::Text {
                text: "hello".into(),
            }],
        }],
        tools: vec![],
        input_token_budget: 10,
        max_output_tokens: 20,
        reasoning: None,
        streaming: false,
    }
}
fn response() -> ProviderResponse {
    let request = request();
    ProviderResponse {
        schema_version: 1,
        request_id: request.request_id,
        provider_id: request.provider_id,
        model_id: request.model_id,
        text: "hello".into(),
        tool_calls: vec![],
        finish_reason: FinishReason::Stop,
        usage: TokenUsage::Unknown {
            reason: UsageUnknownReason::NotReported,
        },
    }
}

#[test]
fn native_api_requires_matching_credential_and_explicit_metered_entitlement() {
    let (profile, connection, credential, entitlement, model) = fixtures();
    let binding = validate_binding(
        &profile,
        &connection,
        Some(&credential),
        &entitlement,
        &model,
        &ProviderPolicy::default(),
        Timestamp(1),
    )
    .unwrap();
    assert_eq!(binding.credential_reference(), Some(&credential.id));
    assert_eq!(binding.adapter_id(), &connection.adapter);
    assert!(
        !serde_json::to_string(&binding)
            .unwrap()
            .contains(&credential.vault_key)
    );
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            None,
            &entitlement,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::CredentialRequired
    );
    let mut wrong = credential;
    wrong.id = CredentialReferenceId::new("other").unwrap();
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            Some(&wrong),
            &entitlement,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::BindingMismatch
    );
}

#[test]
fn external_harness_subscription_never_accepts_copied_native_credentials() {
    let (mut profile, mut connection, credential, mut entitlement, model) = fixtures();
    profile.runtime = RuntimeKind::ExternalHarness;
    profile.adapter = AgentRuntimeAdapterId::new("external-loop").unwrap();
    profile.installation = Some(InstallationId::new("external-cli").unwrap());
    connection.authentication = AuthenticationKind::HarnessManaged;
    entitlement.kind = BillingKind::HarnessSubscription;
    let binding = validate_binding(
        &profile,
        &connection,
        None,
        &entitlement,
        &model,
        &ProviderPolicy::default(),
        Timestamp(1),
    )
    .unwrap();
    assert_eq!(binding.runtime_kind(), RuntimeKind::ExternalHarness);
    assert_eq!(binding.credential_reference(), None);
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            Some(&credential),
            &entitlement,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::UnexpectedCredential
    );
    profile.runtime = RuntimeKind::NativeSymbiote;
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            None,
            &entitlement,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::UnsupportedAuthenticationBilling
    );
}

#[test]
fn native_local_endpoint_needs_no_cli_or_secret_but_requires_exact_local_policy() {
    let (profile, mut connection, _, mut entitlement, model) = fixtures();
    connection.authentication = AuthenticationKind::LocalUnauthenticated;
    entitlement.kind = BillingKind::Local;
    assert!(profile.installation.is_none());
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            None,
            &entitlement,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::LocalPolicyRequired
    );
    let policy = ProviderPolicy {
        verified_local_endpoints: BTreeMap::from([(
            connection.id.clone(),
            connection.endpoint_reference.clone(),
        )]),
    };
    let binding = validate_binding(
        &profile,
        &connection,
        None,
        &entitlement,
        &model,
        &policy,
        Timestamp(1),
    )
    .unwrap();
    assert_eq!(binding.credential_reference(), None);
    assert_eq!(binding.endpoint_reference(), "endpoint-main");
    connection.endpoint_reference = "different-endpoint".into();
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            None,
            &entitlement,
            &model,
            &policy,
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::LocalPolicyRequired
    );
}

#[test]
fn binding_mismatches_expiry_and_billing_fallback_fail_closed() {
    let (profile, connection, credential, entitlement, model) = fixtures();
    for at in [Timestamp(100), Timestamp(101)] {
        assert_eq!(
            validate_binding(
                &profile,
                &connection,
                Some(&credential),
                &entitlement,
                &model,
                &ProviderPolicy::default(),
                at
            )
            .unwrap_err(),
            ProviderError::ExpiredEntitlement
        );
    }
    let binding = validate_binding(
        &profile,
        &connection,
        Some(&credential),
        &entitlement,
        &model,
        &ProviderPolicy::default(),
        Timestamp(99),
    )
    .unwrap();
    assert!(binding.validate_at(Timestamp(100)).is_err());
    let mut mismatch = entitlement.clone();
    mismatch.provider = ProviderConnectionId::new("other").unwrap();
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            Some(&credential),
            &mismatch,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::BindingMismatch
    );
    let mut mismatch = entitlement;
    mismatch.kind = BillingKind::HarnessSubscription;
    assert_eq!(
        validate_binding(
            &profile,
            &connection,
            Some(&credential),
            &mismatch,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .unwrap_err(),
        ProviderError::UnsupportedAuthenticationBilling
    );
}

#[test]
fn mixed_runtime_profiles_roundtrip_without_changing_role_identity() {
    let (native, mut connection, _, mut entitlement, model) = fixtures();
    let mut external = native.clone();
    external.runtime = RuntimeKind::ExternalHarness;
    external.id = RuntimeProfileId::new("external-profile").unwrap();
    external.adapter = AgentRuntimeAdapterId::new("external-loop").unwrap();
    connection.authentication = AuthenticationKind::HarnessManaged;
    entitlement.kind = BillingKind::HarnessSubscription;
    let lead = RoleId::new("lead").unwrap();
    let worker = RoleId::new("worker").unwrap();
    for assignments in [
        vec![
            (lead.clone(), native.clone()),
            (worker.clone(), external.clone()),
        ],
        vec![
            (lead.clone(), external.clone()),
            (worker.clone(), native.clone()),
        ],
        vec![
            (lead.clone(), native.clone()),
            (worker.clone(), native.clone()),
        ],
    ] {
        let decoded: Vec<(RoleId, RuntimeProfile)> =
            serde_json::from_str(&serde_json::to_string(&assignments).unwrap()).unwrap();
        assert_eq!(assignments, decoded);
    }
    assert!(
        validate_binding(
            &external,
            &connection,
            None,
            &entitlement,
            &model,
            &ProviderPolicy::default(),
            Timestamp(1)
        )
        .is_ok()
    );
}

#[test]
fn ultra_requires_explicit_model_advertisement() {
    let (_, _, _, _, mut model) = fixtures();
    let mut req = request();
    req.reasoning = Some(ReasoningEffort::Ultra);
    assert_eq!(
        req.validate(&model),
        Err(ProviderError::UnsupportedCapability)
    );
    model
        .capabilities
        .reasoning_efforts
        .insert(ReasoningEffort::Ultra);
    assert!(req.validate(&model).is_ok());
    assert_eq!(
        serde_json::to_string(&ReasoningEffort::Ultra).unwrap(),
        "\"ultra\""
    );
    req.reasoning = Some(ReasoningEffort::None);
    assert_eq!(
        req.validate(&model),
        Err(ProviderError::UnsupportedCapability)
    );
    req.reasoning = None;
    assert!(req.validate(&model).is_ok());
}

#[test]
fn model_context_and_feature_requests_are_explicit() {
    let (_, _, _, _, mut model) = fixtures();
    assert!(request().validate(&model).is_ok());
    let mut req = request();
    req.input_token_budget = 109;
    assert_eq!(req.validate(&model), Err(ProviderError::ContextLimit));
    req.input_token_budget = u64::MAX;
    assert_eq!(req.validate(&model), Err(ProviderError::ContextLimit));
    let mut req = request();
    req.max_output_tokens = 65;
    assert_eq!(req.validate(&model), Err(ProviderError::OutputLimit));
    let mut req = request();
    req.reasoning = Some(ReasoningEffort::High);
    assert_eq!(
        req.validate(&model),
        Err(ProviderError::UnsupportedCapability)
    );
    model.capabilities.streaming = false;
    let mut req = request();
    req.streaming = true;
    assert_eq!(
        req.validate(&model),
        Err(ProviderError::UnsupportedCapability)
    );
    model.capabilities.images = false;
    let mut req = request();
    req.messages[0].content = vec![InputPart::Image {
        artifact_id: ArtifactId::new("image").unwrap(),
    }];
    assert_eq!(
        req.validate(&model),
        Err(ProviderError::UnsupportedCapability)
    );
}

#[test]
fn unknown_usage_is_never_zero_and_known_subsets_do_not_double_count() {
    for reason in [
        UsageUnknownReason::NotReported,
        UsageUnknownReason::Interrupted,
        UsageUnknownReason::Unsupported,
    ] {
        assert_eq!(TokenUsage::Unknown { reason }.total().unwrap(), None);
    }
    assert_eq!(
        TokenUsage::Known {
            input_tokens: 10,
            output_tokens: 20,
            cached_input_tokens: Some(5),
            reasoning_tokens: Some(12)
        }
        .total()
        .unwrap(),
        Some(30)
    );
    for usage in [
        TokenUsage::Known {
            input_tokens: 1,
            output_tokens: 1,
            cached_input_tokens: Some(2),
            reasoning_tokens: None,
        },
        TokenUsage::Known {
            input_tokens: 1,
            output_tokens: 1,
            cached_input_tokens: None,
            reasoning_tokens: Some(2),
        },
        TokenUsage::Known {
            input_tokens: u64::MAX,
            output_tokens: 1,
            cached_input_tokens: None,
            reasoning_tokens: None,
        },
    ] {
        assert_eq!(usage.total(), Err(ProviderError::InvalidUsage));
    }
}

#[test]
fn tool_calls_require_declared_names_unique_ids_and_object_arguments() {
    let (_, _, _, _, model) = fixtures();
    let mut req = request();
    req.tools.push(ToolDefinition {
        name: "read_code".into(),
        description: "Read source".into(),
        input_schema: json!({"type":"object"}),
    });
    let mut output = response();
    output.finish_reason = FinishReason::ToolCalls;
    output.tool_calls.push(ProviderToolCall {
        call_id: RequestId::new("tool-call").unwrap(),
        name: "read_code".into(),
        arguments: json!({"path":"example.rs"}),
    });
    assert!(output.validate(&req, &model).is_ok());
    output.tool_calls.push(output.tool_calls[0].clone());
    assert_eq!(
        output.validate(&req, &model),
        Err(ProviderError::DuplicateToolCall)
    );
    output.tool_calls.pop();
    output.tool_calls[0].name = "execute_anything".into();
    assert_eq!(
        output.validate(&req, &model),
        Err(ProviderError::UnknownTool)
    );
    output.tool_calls[0].name = "read_code".into();
    output.tool_calls[0].arguments = json!([]);
    assert_eq!(
        output.validate(&req, &model),
        Err(ProviderError::InvalidEnvelope)
    );
}

#[test]
fn envelopes_are_bounded_strict_and_correlated() {
    let (_, _, _, _, model) = fixtures();
    let req = request();
    assert_eq!(
        ProviderRequest::parse(&serde_json::to_vec(&req).unwrap(), &model).unwrap(),
        req
    );
    let mut unknown = serde_json::to_value(&req).unwrap();
    unknown["credential_value"] = json!("must-not-be-here");
    assert_eq!(
        ProviderRequest::parse(unknown.to_string().as_bytes(), &model).unwrap_err(),
        ProviderError::InvalidEnvelope
    );
    assert_eq!(
        ProviderRequest::parse(&vec![b' '; MAX_ENVELOPE_BYTES + 1], &model).unwrap_err(),
        ProviderError::EnvelopeTooLarge
    );
    let mut output = response();
    output.request_id = RequestId::new("other").unwrap();
    assert_eq!(
        output.validate(&req, &model),
        Err(ProviderError::ModelMismatch)
    );
    let mut output = response();
    output.text = "a".repeat(MAX_ENVELOPE_BYTES);
    assert_eq!(
        output.validate(&req, &model),
        Err(ProviderError::EnvelopeTooLarge)
    );
    let mut bad_version = req.clone();
    bad_version.schema_version = 2;
    assert_eq!(
        bad_version.validate(&model),
        Err(ProviderError::UnsupportedVersion)
    );
}

#[test]
fn known_input_usage_respects_budget_even_when_model_context_has_room() {
    let (_, _, _, _, model) = fixtures();
    let req = request();
    let mut output = response();
    for input_tokens in [9, 10, 11, 100] {
        output.usage = TokenUsage::Known {
            input_tokens,
            output_tokens: 1,
            cached_input_tokens: None,
            reasoning_tokens: None,
        };
        let expected = if input_tokens <= req.input_token_budget {
            Ok(())
        } else {
            Err(ProviderError::InputLimit)
        };
        assert_eq!(output.validate(&req, &model), expected);
    }
    output.usage = TokenUsage::Unknown {
        reason: UsageUnknownReason::NotReported,
    };
    assert!(output.validate(&req, &model).is_ok());
    assert_eq!(output.usage.total().unwrap(), None);
}

#[test]
fn nested_tool_json_and_usage_overruns_are_rejected() {
    let (_, _, _, _, model) = fixtures();
    let mut req = request();
    let mut schema = json!({});
    for _ in 0..34 {
        schema = json!({"child":schema});
    }
    req.tools.push(ToolDefinition {
        name: "tool".into(),
        description: String::new(),
        input_schema: schema,
    });
    assert_eq!(req.validate(&model), Err(ProviderError::InvalidEnvelope));
    let mut output = response();
    output.usage = TokenUsage::Known {
        input_tokens: 10,
        output_tokens: 21,
        cached_input_tokens: None,
        reasoning_tokens: None,
    };
    assert_eq!(
        output.validate(&request(), &model),
        Err(ProviderError::OutputLimit)
    );
    output.usage = TokenUsage::Known {
        input_tokens: 128,
        output_tokens: 1,
        cached_input_tokens: None,
        reasoning_tokens: None,
    };
    assert_eq!(
        output.validate(&request(), &model),
        Err(ProviderError::ContextLimit)
    );
}
