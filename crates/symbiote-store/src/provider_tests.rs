use super::*;

fn project(store: &mut Store, tag: &str) -> ProjectId {
    let (project, roots, roles) = records(&format!("provider-{tag}"));
    store
        .register_project(
            id!(CommandId, &format!("register-{tag}")),
            project.clone(),
            roots,
            roles,
        )
        .unwrap();
    project.id
}

fn provider_record(id: &str, adapter: &str, endpoint: &str) -> ProviderConnection {
    ProviderConnection {
        id: id!(ProviderConnectionId, id),
        adapter: id!(InferenceProviderAdapterId, adapter),
        endpoint_reference: endpoint.to_owned(),
        authentication: AuthenticationKind::ApiCredential,
    }
}

fn entitlement(id: &str, provider: &str) -> BillingEntitlement {
    BillingEntitlement {
        id: id!(BillingEntitlementId, id),
        provider: id!(ProviderConnectionId, provider),
        kind: BillingKind::MeteredApi,
        verification_evidence: id!(EvidenceId, "entitlement-proof"),
        expires_at: Timestamp(1_000_000),
    }
}

fn model(
    id: &str,
    provider: &str,
    context: u64,
) -> symbiote_runtime_sdk::provider::ModelDescriptor {
    symbiote_runtime_sdk::provider::ModelDescriptor {
        schema_version: symbiote_runtime_sdk::provider::PROVIDER_CONTRACT_VERSION,
        id: id!(ModelId, id),
        provider_id: id!(ProviderConnectionId, provider),
        context_window_tokens: context,
        max_output_tokens: context / 2,
        capabilities: symbiote_runtime_sdk::provider::ModelCapabilities {
            reasoning_efforts: BTreeSet::from([
                symbiote_runtime_sdk::provider::ReasoningEffort::Medium,
            ]),
            tools: true,
            images: false,
            streaming: true,
        },
    }
}

#[test]
fn provider_records_register_read_and_replay_across_restart() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let attribution = project(&mut store, "one");
    let connection = provider_record("openai", "openai-adapter", "https://api.openai.example/v1");
    store
        .replace_provider_connection(
            id!(CommandId, "provider-1"),
            attribution.clone(),
            connection.clone(),
            id!(UserId, "owner"),
            Timestamp(100),
        )
        .unwrap();
    // Replay of the identical command returns the original receipt.
    let replay = store
        .replace_provider_connection(
            id!(CommandId, "provider-1"),
            attribution.clone(),
            connection.clone(),
            id!(UserId, "owner"),
            Timestamp(100),
        )
        .unwrap();
    assert!(replay.replayed);
    // Replacement supersedes: a new endpoint on the same identity.
    let replaced = provider_record("openai", "openai-adapter", "https://api2.openai.example/v1");
    store
        .replace_provider_connection(
            id!(CommandId, "provider-2"),
            attribution.clone(),
            replaced,
            id!(UserId, "owner"),
            Timestamp(101),
        )
        .unwrap();
    // Entitlements and models require the connection to exist first.
    assert!(matches!(
        store.replace_billing_entitlement(
            id!(CommandId, "entitlement-early"),
            attribution.clone(),
            entitlement("ent-openai", "ghost-provider"),
            id!(UserId, "owner"),
            Timestamp(102),
        ),
        Err(StoreError::RelationshipMismatch)
    ));
    store
        .replace_billing_entitlement(
            id!(CommandId, "entitlement-1"),
            attribution.clone(),
            entitlement("ent-openai", "openai"),
            id!(UserId, "owner"),
            Timestamp(103),
        )
        .unwrap();
    store
        .replace_model_descriptor(
            id!(CommandId, "model-1"),
            attribution.clone(),
            model("gpt-fixture", "openai", 128_000),
            id!(UserId, "owner"),
            Timestamp(104),
        )
        .unwrap();
    // Invalid model descriptors are refused by the SDK contract.
    assert!(matches!(
        store.replace_model_descriptor(
            id!(CommandId, "model-bad"),
            attribution.clone(),
            model("gpt-bad", "openai", 0),
            id!(UserId, "owner"),
            Timestamp(105),
        ),
        Err(StoreError::InvalidProvider)
    ));
    drop(store);
    let store = Store::open(temp.database()).unwrap();
    assert_eq!(
        store
            .provider_connection(&id!(ProviderConnectionId, "openai"))
            .unwrap()
            .endpoint_reference,
        "https://api2.openai.example/v1"
    );
    assert_eq!(
        store
            .billing_entitlement(&id!(BillingEntitlementId, "ent-openai"))
            .unwrap()
            .expires_at,
        Timestamp(1_000_000)
    );
    assert_eq!(
        store
            .model_descriptor(&id!(ModelId, "gpt-fixture"))
            .unwrap()
            .context_window_tokens,
        128_000
    );
    assert_eq!(
        store
            .provider_command_timestamp(&id!(CommandId, "provider-1"))
            .unwrap(),
        Some(Timestamp(100))
    );
    assert!(matches!(
        store.provider_connection(&id!(ProviderConnectionId, "missing")),
        Err(StoreError::NotFound)
    ));
}

#[test]
fn tampered_provider_rows_are_refused_on_reopen() {
    let temp = Temporary::new();
    let mut store = Store::open(temp.database()).unwrap();
    let attribution = project(&mut store, "two");
    store
        .replace_provider_connection(
            id!(CommandId, "provider-1"),
            attribution,
            provider_record("openai", "openai-adapter", "https://api.openai.example/v1"),
            id!(UserId, "owner"),
            Timestamp(100),
        )
        .unwrap();
    drop(store);
    // Desynchronize the indexed adapter column from the body.
    let raw = Connection::open(temp.database()).unwrap();
    raw.execute(
        "UPDATE provider_connections SET adapter='other-adapter' WHERE id='openai'",
        [],
    )
    .unwrap();
    drop(raw);
    assert!(matches!(
        Store::open(temp.database()),
        Err(StoreError::Integrity(_))
    ));
}
