fn main() -> Result<(), serde_json::Error> {
    let schemas = serde_json::json!({
        "inventory": schemars::schema_for!(symbiote_runtime_discovery::Inventory),
        "eligibility_query": schemars::schema_for!(symbiote_runtime_discovery::EligibilityQuery),
        "installation": schemars::schema_for!(symbiote_runtime_discovery::ExecutableInstallation),
        "profile_spec": schemars::schema_for!(symbiote_runtime_discovery::ProfileSpec),
        "profile_observation": schemars::schema_for!(symbiote_runtime_discovery::ProfileObservation),
        "diagnostic": schemars::schema_for!(symbiote_runtime_discovery::RuntimeDiagnostic)
    });
    println!("{}", serde_json::to_string_pretty(&schemas)?);
    Ok(())
}
