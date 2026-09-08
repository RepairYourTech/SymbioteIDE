fn main() {
    let schemas = serde_json::json!({
        "runtime_descriptor": schemars::schema_for!(symbiote_runtime_sdk::RuntimeDescriptor),
        "runtime_event": schemars::schema_for!(symbiote_runtime_sdk::events::RuntimeEvent),
        "provider_request": schemars::schema_for!(symbiote_runtime_sdk::provider::ProviderRequest),
        "provider_response": schemars::schema_for!(symbiote_runtime_sdk::provider::ProviderResponse)
    });
    println!("{}", serde_json::to_string_pretty(&schemas).unwrap());
}
