//! A deterministic inactive profile publication; no real harness configuration.
use std::{collections::BTreeMap, path::Path};
use symbiote_projection::{generation, toml::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: projection_demo PRIVATE_ROOT NEW_GENERATION".into());
    }
    let base = "enabled = false\nuser_setting = 1\n";
    let actual = "# user comment\nenabled = false # retain\nuser_setting = 2\n";
    let plan = plan(
        base,
        actual,
        &[ManagedChange {
            path: vec!["enabled".into()],
            before: Some(PrimitiveValue::Boolean(false)),
            desired: Some(PrimitiveValue::Boolean(true)),
            ownership: ManagedOwnership::SymbioteManaged,
        }],
    )?;
    let files = BTreeMap::from([("config.toml".into(), plan.candidate.into_bytes())]);
    let receipt = generation::publish(Path::new(&args[0]), &args[1], &files)?;
    assert_eq!(generation::verify(Path::new(&args[0]), &args[1])?, receipt);
    println!(
        "Verified an inactive fixture generation with one TOML file. User comments and unmanaged edits are preserved. No runtime was activated."
    );
    Ok(())
}
