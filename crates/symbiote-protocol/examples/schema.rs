fn main() {
    let schemas = serde_json::json!({
        "request": schemars::schema_for!(symbiote_protocol::Request),
        "response": schemars::schema_for!(symbiote_protocol::Response),
        "telemetry": schemars::schema_for!(symbiote_protocol::Telemetry),
    });
    println!("{}", serde_json::to_string_pretty(&schemas).unwrap());
}
