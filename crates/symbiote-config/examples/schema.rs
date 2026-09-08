fn main() {
    let schemas = serde_json::json!({
        "project_manifest": schemars::schema_for!(symbiote_config::ProjectManifest),
        "configuration_layer": schemars::schema_for!(symbiote_config::Layer),
        "machine_bindings": schemars::schema_for!(symbiote_config::MachineBindings),
        "agent_environment": schemars::schema_for!(symbiote_config::environment::EnvironmentDocument),
    });
    println!("{}", serde_json::to_string_pretty(&schemas).unwrap());
}
