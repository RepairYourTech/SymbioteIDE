//! Read-only plan inspection. It never fabricates compatibility or permission facts.
use std::{collections::BTreeSet, env, fs};
use symbiote_config::{ProjectManifest, environment::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: environment_plan PROJECT_MANIFEST ENVIRONMENT_NAME TARGET_JSON".into());
    }
    // Bound reads before parsing; error messages contain no configuration payload.
    let manifest =
        ProjectManifest::parse(&read_bounded(&args[0])?).map_err(|_| "invalid Project manifest")?;
    let document = manifest
        .agent_environments
        .get(&args[1])
        .ok_or("unknown environment")?;
    let target = ResolutionTarget::parse(&read_bounded(&args[2])?).map_err(|_| "invalid target")?;
    // No probe/Host authorization is available in this standalone inspector.
    // Missing facts therefore remain blockers or excluded optional resources.
    let facts = ResolutionFacts {
        target: target.clone(),
        mode: document.mode,
        mode_supported: false,
        resources: Vec::new(),
        approved_core_policies: BTreeSet::new(),
        approved_extensions: BTreeSet::new(),
        grants: BTreeSet::new(),
        parent_grants: None,
    };
    let plan = document.resolve(&target, &facts)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    eprintln!(
        "Read-only intent inspection. Compatibility, permissions and profile mode are unverified; this output cannot authorize projection or execution."
    );
    Ok(())
}

fn read_bounded(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("input exceeds size limit".into());
    }
    String::from_utf8(bytes).map_err(|_| "input must be UTF-8".into())
}
