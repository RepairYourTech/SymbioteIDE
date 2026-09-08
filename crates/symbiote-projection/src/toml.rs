//! Pure reconciliation of explicitly owned primitive TOML keys.
use std::fmt;
use std::fmt::Write;
use toml_edit::{DocumentMut, Item, Table, Value};

const MAX_BYTES: usize = 1_048_576;

struct BoundedOutput(String);
impl fmt::Write for BoundedOutput {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if value.len() > MAX_BYTES.saturating_sub(self.0.len()) {
            return Err(fmt::Error);
        }
        self.0.push_str(value);
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum PrimitiveValue {
    String(String),
    Boolean(bool),
    Integer(i64),
}
impl fmt::Debug for PrimitiveValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrimitiveValue([redacted])")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedOwnership {
    SymbioteManaged,
    Unmanaged,
}
#[derive(Clone, PartialEq, Eq)]
pub struct ManagedChange {
    pub path: Vec<String>,
    pub before: Option<PrimitiveValue>,
    pub desired: Option<PrimitiveValue>,
    pub ownership: ManagedOwnership,
}
impl fmt::Debug for ManagedChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ManagedChange([redacted])")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    LimitExceeded,
    MalformedToml,
    InvalidPath,
    OverlappingPaths,
    UnmanagedKey,
    BaselineMismatch,
    ConcurrentEdit,
    UnsupportedValue,
    UnsupportedCommentedDeletion,
}
impl fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TOML reconciliation refused")
    }
}
impl std::error::Error for ProjectionError {}

#[derive(Clone, PartialEq, Eq)]
pub struct SemanticDiff {
    pub path: Vec<String>,
    pub before: Option<PrimitiveValue>,
    pub after: Option<PrimitiveValue>,
}
impl fmt::Debug for SemanticDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SemanticDiff([redacted])")
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct TomlPlan {
    pub candidate: String,
    pub diffs: Vec<SemanticDiff>,
}
impl fmt::Debug for TomlPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TomlPlan")
            .field("candidate", &"[redacted]")
            .field("diff_count", &self.diffs.len())
            .finish()
    }
}

fn read(table: &Table, path: &[String]) -> Result<Option<PrimitiveValue>, ProjectionError> {
    let Some(item) = table.get(&path[0]) else {
        return Ok(None);
    };
    if path.len() > 1 {
        return read(
            item.as_table().ok_or(ProjectionError::UnsupportedValue)?,
            &path[1..],
        );
    }
    match item.as_value() {
        Some(Value::String(value)) => Ok(Some(PrimitiveValue::String(value.value().clone()))),
        Some(Value::Boolean(value)) => Ok(Some(PrimitiveValue::Boolean(*value.value()))),
        Some(Value::Integer(value)) => Ok(Some(PrimitiveValue::Integer(*value.value()))),
        _ => Err(ProjectionError::UnsupportedValue),
    }
}
fn value(primitive: &PrimitiveValue) -> Value {
    match primitive {
        PrimitiveValue::String(v) => Value::from(v.as_str()),
        PrimitiveValue::Boolean(v) => Value::from(*v),
        PrimitiveValue::Integer(v) => Value::from(*v),
    }
}

fn edit(
    table: &mut Table,
    path: &[String],
    desired: &Option<PrimitiveValue>,
) -> Result<(), ProjectionError> {
    let key = &path[0];
    if path.len() > 1 {
        if !table.contains_key(key) {
            let mut child = Table::new();
            child.set_implicit(true);
            table.insert(key, Item::Table(child));
        }
        return edit(
            table
                .get_mut(key)
                .and_then(Item::as_table_mut)
                .ok_or(ProjectionError::UnsupportedValue)?,
            &path[1..],
            desired,
        );
    }
    match desired {
        Some(primitive) => {
            let mut replacement = value(primitive);
            if let Some(previous) = table.get(key).and_then(Item::as_value) {
                *replacement.decor_mut() = previous.decor().clone();
            }
            if let Some(existing) = table.get_mut(key) {
                *existing = Item::Value(replacement);
            } else {
                table.insert(key, Item::Value(replacement));
            }
        }
        None => {
            // Deletion cannot silently discard either key-prefix or value-suffix comments.
            if let Some((formatted_key, item)) = table.get_key_value(key) {
                let has_comment = |raw: Option<&toml_edit::RawString>| {
                    raw.and_then(toml_edit::RawString::as_str)
                        .is_some_and(|s| s.contains('#'))
                };
                if has_comment(formatted_key.leaf_decor().prefix())
                    || has_comment(formatted_key.leaf_decor().suffix())
                    || item.as_value().is_some_and(|v| {
                        has_comment(v.decor().prefix()) || has_comment(v.decor().suffix())
                    })
                {
                    return Err(ProjectionError::UnsupportedCommentedDeletion);
                }
            }
            table.remove(key);
        }
    }
    Ok(())
}

/// Caller supplies ownership from its trusted managed-key ledger. This function
/// never discovers/adopts existing ownership, reads files, or executes TOML content.
pub fn plan(
    base: &str,
    current: &str,
    changes: &[ManagedChange],
) -> Result<TomlPlan, ProjectionError> {
    if base.len() > MAX_BYTES || current.len() > MAX_BYTES || changes.len() > 1024 {
        return Err(ProjectionError::LimitExceeded);
    }
    let base = base
        .parse::<DocumentMut>()
        .map_err(|_| ProjectionError::MalformedToml)?;
    let mut candidate = current
        .parse::<DocumentMut>()
        .map_err(|_| ProjectionError::MalformedToml)?;
    let mut ordered: Vec<_> = changes.iter().collect();
    ordered.sort_by(|a, b| a.path.cmp(&b.path));
    let mut aggregate_values = 0_usize;
    for change in &ordered {
        if change.path.is_empty()
            || change.path.len() > 16
            || change
                .path
                .iter()
                .any(|p| p.is_empty() || p.len() > 128 || p.chars().any(char::is_control))
        {
            return Err(ProjectionError::InvalidPath);
        }
        if change.ownership != ManagedOwnership::SymbioteManaged {
            return Err(ProjectionError::UnmanagedKey);
        }
        for value in [&change.before, &change.desired] {
            if let Some(PrimitiveValue::String(value)) = value {
                aggregate_values = aggregate_values
                    .checked_add(value.len())
                    .ok_or(ProjectionError::LimitExceeded)?;
                if aggregate_values > MAX_BYTES {
                    return Err(ProjectionError::LimitExceeded);
                }
            }
        }
    }
    for pair in ordered.windows(2) {
        if pair[1].path.starts_with(&pair[0].path) {
            return Err(ProjectionError::OverlappingPaths);
        }
    }
    let mut diffs = Vec::new();
    for change in ordered {
        let baseline = read(base.as_table(), &change.path)?;
        if baseline != change.before {
            return Err(ProjectionError::BaselineMismatch);
        }
        let actual = read(candidate.as_table(), &change.path)?;
        if actual == change.desired {
            continue;
        }
        if actual != baseline {
            return Err(ProjectionError::ConcurrentEdit);
        }
        edit(candidate.as_table_mut(), &change.path, &change.desired)?;
        diffs.push(SemanticDiff {
            path: change.path.clone(),
            before: actual,
            after: change.desired.clone(),
        });
    }
    let mut output = BoundedOutput(String::new());
    write!(&mut output, "{candidate}").map_err(|_| ProjectionError::LimitExceeded)?;
    let candidate = output.0;
    Ok(TomlPlan { candidate, diffs })
}
