fn main() -> Result<(), serde_json::Error> {
    let schema = schemars::schema_for!(symbiote_domain::DomainEnvelope);
    println!("{}", serde_json::to_string_pretty(&schema)?);
    Ok(())
}
