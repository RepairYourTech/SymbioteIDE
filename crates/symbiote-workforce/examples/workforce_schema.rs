fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(
            symbiote_workforce::BindingConfiguration
        ))
        .expect("schema serialization")
    );
}
