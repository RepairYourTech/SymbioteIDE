fn main() -> Result<(), serde_json::Error> {
    let schemas = serde_json::json!({"inventory":schemars::schema_for!(symbiote_runtime_discovery::Inventory),"eligibility_query":schemars::schema_for!(symbiote_runtime_discovery::EligibilityQuery)});
    println!("{}", serde_json::to_string_pretty(&schemas)?);
    Ok(())
}
