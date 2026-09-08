fn main() -> Result<(), serde_json::Error> {
    let schemas = serde_json::json!({
        "work_item": schemars::schema_for!(symbiote_domain::WorkItem),
        "work_command": schemars::schema_for!(symbiote_domain::WorkCommand),
        "task_origin": schemars::schema_for!(symbiote_domain::TaskOrigin),
    });
    println!("{}", serde_json::to_string_pretty(&schemas)?);
    Ok(())
}
