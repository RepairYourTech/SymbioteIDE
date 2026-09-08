fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&schemars::schema_for!(symbiote_host_inventory::HostPulse))
            .expect("schema serialization")
    );
}
