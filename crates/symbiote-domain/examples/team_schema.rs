fn main() -> Result<(), serde_json::Error> {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(symbiote_domain::TeamConfiguration))?
    );
    Ok(())
}
