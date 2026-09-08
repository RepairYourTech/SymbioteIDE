//! Pure desired-state preparation followed by optional inactive profile publication.
//! This crate does not authorize resource activation or discover native config paths.
pub mod generation;
pub mod toml;

use std::collections::{BTreeMap, BTreeSet};
use symbiote_config::environment::{EffectiveEnvironment, EnvironmentResource, Requirement};
use toml::ManagedChange;

/// A pack's explicit native-file mapping. Metadata alone is not a compatibility proof.
pub struct TomlDeclaration {
    pub resource: EnvironmentResource,
    pub filename: String,
    pub changes: Vec<ManagedChange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparationError {
    BlockedEnvironment,
    IneligibleResource,
    MissingInput,
    MissingMapping,
    LimitExceeded,
    Reconciliation(toml::ProjectionError),
}

pub struct PreparedFiles {
    files: BTreeMap<String, Vec<u8>>,
}
impl PreparedFiles {
    /// Private configuration content: do not log or serialize without consent.
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }
}

/// Combine explicit pack mappings with eligible environment intent. Inputs are
/// private, non-credential configuration supplied by the owning pack. No native
/// paths are inferred, and no file is read or written by this function.
pub fn prepare_toml(
    environment: &EffectiveEnvironment,
    baselines: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
    declarations: &[TomlDeclaration],
) -> Result<PreparedFiles, PreparationError> {
    if !environment.blockers.is_empty() {
        return Err(PreparationError::BlockedEnvironment);
    }
    if declarations.len() > 128 {
        return Err(PreparationError::LimitExceeded);
    }
    let mut grouped: BTreeMap<String, Vec<ManagedChange>> = BTreeMap::new();
    let mut mapped = BTreeSet::new();
    let mut change_count = 0usize;
    let mut value_bytes = 0usize;
    for declaration in declarations {
        if !environment
            .resources
            .get(&declaration.resource.id)
            .is_some_and(|resource| {
                resource.is_eligible() && resource.resource == declaration.resource
            })
        {
            return Err(PreparationError::IneligibleResource);
        }
        if declaration.changes.len() > 128 || declaration.filename.len() > 128 {
            return Err(PreparationError::LimitExceeded);
        }
        if declaration.changes.is_empty() {
            return Err(PreparationError::MissingMapping);
        }
        change_count += declaration.changes.len();
        if change_count > 1024 {
            return Err(PreparationError::LimitExceeded);
        }
        for change in &declaration.changes {
            if change.path.len() > 16 || change.path.iter().any(|part| part.len() > 128) {
                return Err(PreparationError::LimitExceeded);
            }
            for value in [&change.before, &change.desired] {
                if let Some(toml::PrimitiveValue::String(value)) = value {
                    value_bytes = value_bytes
                        .checked_add(value.len())
                        .ok_or(PreparationError::LimitExceeded)?;
                    if value_bytes > 1_048_576 {
                        return Err(PreparationError::LimitExceeded);
                    }
                }
            }
        }
        mapped.insert(&declaration.resource.id);
        grouped
            .entry(declaration.filename.clone())
            .or_default()
            .extend(declaration.changes.clone());
    }
    if environment.resources.iter().any(|(id, resource)| {
        resource.resource.requirement == Requirement::Required && !mapped.contains(id)
    }) {
        return Err(PreparationError::MissingMapping);
    }
    if grouped.len() > 16 {
        return Err(PreparationError::LimitExceeded);
    }
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for (filename, changes) in grouped {
        let baseline = baselines
            .get(&filename)
            .ok_or(PreparationError::MissingInput)?;
        let actual = current
            .get(&filename)
            .ok_or(PreparationError::MissingInput)?;
        let plan =
            toml::plan(baseline, actual, &changes).map_err(PreparationError::Reconciliation)?;
        total = total
            .checked_add(plan.candidate.len())
            .ok_or(PreparationError::LimitExceeded)?;
        if total > 4 * 1_048_576 {
            return Err(PreparationError::LimitExceeded);
        }
        files.insert(filename, plan.candidate.into_bytes());
    }
    Ok(PreparedFiles { files })
}
