fn main() {
    let schemas = serde_json::json!({
        "decision": schemars::schema_for!(symbiote_architecture::Decision),
        "compatibility_fact": schemars::schema_for!(symbiote_architecture::CompatibilityFact),
        "implementation_pins": schemars::schema_for!(symbiote_architecture::ImplementationPins),
        "spike_contract": schemars::schema_for!(symbiote_architecture::SpikeContract),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&schemas).expect("JSON schema serialization")
    );
}
