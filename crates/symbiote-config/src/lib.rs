//! Pure configuration contracts. No filesystem, authentication, or execution effects.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub use symbiote_domain::RuntimeKind;

pub mod environment;
mod json;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_MANIFEST_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub path: String,
    pub message: String,
}

fn diagnostic(path: &str, message: &str) -> Diagnostic {
    Diagnostic {
        path: path.into(),
        message: message.into(),
    }
}

/// Increasing specificity; machine and secret state are intentionally outside inheritance.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Application,
    User,
    Project,
    WorkspaceRoot,
    Repository,
    Role,
    LocalOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StorageClass {
    VersionControlled,
    UserSynchronized,
    LocalOnly,
    ProhibitedFromDisk,
}

pub fn scope_storage(scope: Scope) -> StorageClass {
    match scope {
        Scope::Application
        | Scope::Project
        | Scope::WorkspaceRoot
        | Scope::Repository
        | Scope::Role => StorageClass::VersionControlled,
        Scope::User => StorageClass::UserSynchronized,
        Scope::LocalOverride => StorageClass::LocalOnly,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub project: String,
    pub root: Option<String>,
    pub repository: Option<String>,
    pub role: Option<String>,
    pub controller: String,
}

/// Typed policy values avoid a generic key/value channel for credentials.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub learning: Option<bool>,
    pub telemetry: Option<bool>,
    pub retention_days: Option<u32>,
    pub delegation: Option<bool>,
    pub goal_execution: Option<bool>,
    pub analytics: Option<AnalyticsView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsView {
    Project,
    AuthorizedProjects,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub schema_version: u32,
    pub scope: Scope,
    /// Stable source identity and revision are retained in resolved provenance.
    pub source: String,
    pub revision: String,
    pub context: Option<Context>,
    pub settings: Settings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub scope: Scope,
    pub source: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSettings {
    pub settings: Settings,
    pub provenance: BTreeMap<String, Vec<Provenance>>,
}

fn applies(layer: &Layer, target: &Context) -> Result<bool, Diagnostic> {
    let scope = layer.scope;
    if matches!(scope, Scope::Application | Scope::User) {
        if layer.context.is_some() {
            return Err(diagnostic(
                "context",
                "global layers must not carry project context",
            ));
        }
        return Ok(true);
    }
    let c = layer
        .context
        .as_ref()
        .ok_or_else(|| diagnostic("context", "scoped layer requires context"))?;
    if c.project.is_empty() {
        return Err(diagnostic("context.project", "project is required"));
    }
    let selector = match scope {
        Scope::WorkspaceRoot => c.root.as_ref(),
        Scope::Repository => c.repository.as_ref(),
        Scope::Role => c.role.as_ref(),
        _ => None,
    };
    if matches!(
        scope,
        Scope::WorkspaceRoot | Scope::Repository | Scope::Role
    ) && selector.is_none_or(|s| s.is_empty())
    {
        return Err(diagnostic(
            "context",
            "scope requires its root, repository, or role selector",
        ));
    }
    Ok(c.project == target.project
        && match scope {
            Scope::WorkspaceRoot => c.root == target.root,
            Scope::Repository => c.repository == target.repository,
            Scope::Role => c.role == target.role,
            Scope::LocalOverride => !c.controller.is_empty() && c == target,
            _ => true,
        })
}

/// Missing fields inherit. Equal-scope unequal values fail; equal values keep all sources.
/// Security permission intersection belongs to the Host, not this preference resolver.
pub fn resolve(layers: &[Layer], context: &Context) -> Result<ResolvedSettings, Diagnostic> {
    let mut selected = Vec::new();
    for layer in layers {
        if layer.schema_version != SCHEMA_VERSION {
            return Err(diagnostic(
                "schema_version",
                "unsupported configuration version",
            ));
        }
        if layer.source.is_empty() || layer.revision.is_empty() {
            return Err(diagnostic(
                "source",
                "source identity and revision are required",
            ));
        }
        if applies(layer, context)? {
            selected.push(layer);
        }
    }
    selected
        .sort_by(|a, b| (a.scope, &a.source, &a.revision).cmp(&(b.scope, &b.source, &b.revision)));
    let mut values: BTreeMap<String, (Scope, Value)> = BTreeMap::new();
    let mut provenance: BTreeMap<String, Vec<Provenance>> = BTreeMap::new();
    for layer in selected {
        let Value::Object(fields) =
            serde_json::to_value(&layer.settings).expect("typed settings serialize")
        else {
            unreachable!()
        };
        for (key, value) in fields {
            if value.is_null() {
                continue;
            }
            if let Some((scope, old)) = values.get(&key) {
                if *scope == layer.scope && old != &value {
                    return Err(diagnostic(
                        &key,
                        "conflicting values at equal scope; resolve the sources explicitly",
                    ));
                }
            }
            values.insert(key.clone(), (layer.scope, value));
            provenance.entry(key).or_default().push(Provenance {
                scope: layer.scope,
                source: layer.source.clone(),
                revision: layer.revision.clone(),
            });
        }
    }
    let settings = serde_json::from_value(Value::Object(
        values.into_iter().map(|(k, (_, v))| (k, v)).collect(),
    ))
    .expect("typed settings resolve");
    Ok(ResolvedSettings {
        settings,
        provenance,
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub skills: BTreeSet<String>,
    pub mcps: BTreeSet<String>,
    pub hooks: BTreeSet<String>,
    pub rules: BTreeSet<String>,
    pub permission_policy_refs: BTreeSet<String>,
}

/// A declaration references stable Role identity; it is not a second Role entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleAssignment {
    pub runtime_kind: RuntimeKind,
    pub runtime_profile_ref: String,
    pub provider_connection_ref: Option<String>,
    pub entitlement_ref: Option<String>,
    pub resources: Resources,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub root_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Policies {
    pub preferences: Settings,
    pub orchestration_ref: Option<String>,
    pub context_ref: Option<String>,
    pub verification_gate_refs: BTreeSet<String>,
    pub documentation_ref: Option<String>,
    pub git_workflow_ref: Option<String>,
    pub delivery_target_refs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub project_id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub roots: BTreeMap<String, String>,
    #[serde(default)]
    pub commands: BTreeMap<String, Command>,
    pub lead_role_ref: Option<String>,
    #[serde(default)]
    pub roles: BTreeMap<String, RoleAssignment>,
    #[serde(default)]
    pub harness_requirements: BTreeSet<String>,
    #[serde(default)]
    pub environment_requirements: BTreeSet<String>,
    /// Canonical runtime intent; native files and local credential bindings are not embedded.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_environments: BTreeMap<String, environment::EnvironmentDocument>,
    #[serde(default)]
    pub policies: Policies,
    pub graph_profile_ref: Option<String>,
    /// Explicit opaque vendor namespaces survive round trips. Never execute their contents.
    #[serde(default)]
    pub extensions: BTreeMap<String, Value>,
    /// Field paths generated by a named source; user fields have no managed entry.
    #[serde(default)]
    pub managed: BTreeMap<String, ManagedField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManagedField {
    pub source: String,
    pub revision: String,
}

fn portable_root(path: &str) -> bool {
    // A single dot names the manifest directory; other paths use canonical components.
    if path == "." {
        return true;
    }
    path.split('/').all(|component| {
        if component.is_empty()
            || component == "."
            || component.ends_with(['.', ' '])
            || component
                .chars()
                .any(|c| c.is_control() || "<>:\"\\|?*".contains(c))
        {
            return false;
        }
        let stem = component
            .split('.')
            .next()
            .unwrap()
            .trim_end()
            .to_ascii_uppercase();
        let reserved = matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
        !reserved
    })
}

impl ProjectManifest {
    pub fn parse(json: &str) -> Result<Self, Diagnostic> {
        let value: Self = crate::json::parse(json, MAX_MANIFEST_BYTES).map_err(|_| {
            diagnostic(
                "manifest",
                "invalid manifest: size, duplicate fields or schema shape rejected",
            )
        })?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(diagnostic(
                "schema_version",
                "unsupported manifest version; migration required",
            ));
        }
        symbiote_domain::ProjectId::new(self.project_id.clone())
            .map_err(|_| diagnostic("project_id", "invalid stable project identity"))?;
        if let Some(role) = &self.lead_role_ref {
            symbiote_domain::RoleId::new(role.clone())
                .map_err(|_| diagnostic("lead_role_ref", "invalid Role reference"))?;
        }
        for (id, path) in &self.roots {
            if id.is_empty() || !portable_root(path) {
                return Err(diagnostic(
                    "roots",
                    "roots require portable relative components: no traversal, control/Windows-invalid characters, reserved device names, or trailing dots/spaces",
                ));
            }
        }
        for command in self.commands.values() {
            if command.program.trim().is_empty() || !self.roots.contains_key(&command.root_ref) {
                return Err(diagnostic(
                    "commands",
                    "commands require a program and declared root",
                ));
            }
        }
        for (role, assignment) in &self.roles {
            symbiote_domain::RoleId::new(role.clone())
                .map_err(|_| diagnostic("roles", "invalid Role reference"))?;
            if assignment.runtime_profile_ref.trim().is_empty()
                || [
                    &assignment.provider_connection_ref,
                    &assignment.entitlement_ref,
                ]
                .iter()
                .any(|reference| reference.as_ref().is_some_and(|s| s.trim().is_empty()))
            {
                return Err(diagnostic(
                    "roles",
                    "resource references must be nonempty when supplied",
                ));
            }
        }
        for (name, environment) in &self.agent_environments {
            if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
                return Err(diagnostic("agent_environments", "invalid environment name"));
            }
            environment.validate().map_err(|_| {
                diagnostic("agent_environments", "invalid desired environment contract")
            })?;
            if environment.project_id.as_str() != self.project_id {
                return Err(diagnostic(
                    "agent_environments",
                    "desired environment belongs to a different Project",
                ));
            }
        }
        for namespace in self.extensions.keys() {
            if !namespace.contains('.') || namespace.split('.').any(str::is_empty) {
                return Err(diagnostic(
                    "extensions",
                    "extension namespace must be dotted, for example org.example",
                ));
            }
        }
        for field in self.managed.values() {
            if field.source.is_empty() || field.revision.is_empty() {
                return Err(diagnostic(
                    "managed",
                    "managed fields require source and revision",
                ));
            }
        }
        Ok(())
    }
    pub fn canonical_json(&self) -> Result<String, Diagnostic> {
        self.validate()?;
        let mut output = ManifestOutput(Vec::new());
        serde_json::to_writer_pretty(&mut output, self)
            .map_err(|_| diagnostic("manifest", "serialized manifest exceeds size limit"))?;
        String::from_utf8(output.0).map_err(|_| diagnostic("manifest", "serialization failed"))
    }
}

struct ManifestOutput(Vec<u8>);
impl std::io::Write for ManifestOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_MANIFEST_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("manifest size limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Machine paths and reference handles are separate, local-only, never embedded in manifests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MachineBindings {
    pub host_ref: String,
    pub project_root_paths: BTreeMap<String, BTreeMap<String, String>>,
    pub provider_credential_refs: BTreeMap<String, String>,
    pub harness_installations: BTreeMap<String, String>,
}
impl MachineBindings {
    pub const STORAGE: StorageClass = StorageClass::LocalOnly;
}

/// Intentionally has no Serialize or JsonSchema implementation; raw secrets cannot use config export.
pub struct SecretValue(Vec<u8>);
impl SecretValue {
    pub const STORAGE: StorageClass = StorageClass::ProhibitedFromDisk;
    pub fn new(value: Vec<u8>) -> Self {
        Self(value)
    }
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ControllerState {
    pub visible_project: Option<String>,
}
impl ControllerState {
    pub const STORAGE: StorageClass = StorageClass::LocalOnly;
    /// Presentation only. There is deliberately no execution handle or cancellation output.
    pub fn switch_project(&mut self, project: String) {
        self.visible_project = Some(project);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeConflict {
    pub path: String,
    pub base: Option<Value>,
    pub left: Option<Value>,
    pub right: Option<Value>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MergePreview {
    pub candidate: Value,
    pub conflicts: Vec<MergeConflict>,
}

fn merge_node(
    path: &str,
    base: Option<&Value>,
    left: Option<&Value>,
    right: Option<&Value>,
    conflicts: &mut Vec<MergeConflict>,
) -> Option<Value> {
    if left == right {
        return left.cloned();
    }
    if left == base {
        return right.cloned();
    }
    if right == base {
        return left.cloned();
    }
    if let (Some(Value::Object(l)), Some(Value::Object(r))) = (left, right) {
        let b = base.and_then(Value::as_object);
        if base.is_none() || b.is_some() {
            let keys: BTreeSet<_> = l
                .keys()
                .chain(r.keys())
                .chain(b.into_iter().flat_map(|o| o.keys()))
                .collect();
            let mut result = serde_json::Map::new();
            for key in keys {
                let child = format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1"));
                if let Some(value) = merge_node(
                    &child,
                    b.and_then(|o| o.get(key)),
                    l.get(key),
                    r.get(key),
                    conflicts,
                ) {
                    result.insert(key.clone(), value);
                }
            }
            return Some(Value::Object(result));
        }
    }
    conflicts.push(MergeConflict {
        path: path.into(),
        base: base.cloned(),
        left: left.cloned(),
        right: right.cloned(),
    });
    base.cloned()
}

/// Conflicting locations keep base solely for display. `finish` refuses unresolved conflicts.
pub fn merge(
    base: &ProjectManifest,
    left: &ProjectManifest,
    right: &ProjectManifest,
) -> Result<MergePreview, Diagnostic> {
    for manifest in [base, left, right] {
        manifest.validate()?;
    }
    if base.project_id != left.project_id || base.project_id != right.project_id {
        return Err(diagnostic("project_id", "cannot merge different projects"));
    }
    let (b, l, r) = (
        serde_json::to_value(base).unwrap(),
        serde_json::to_value(left).unwrap(),
        serde_json::to_value(right).unwrap(),
    );
    let mut conflicts = Vec::new();
    let candidate = merge_node("", Some(&b), Some(&l), Some(&r), &mut conflicts).unwrap();
    Ok(MergePreview {
        candidate,
        conflicts,
    })
}
impl MergePreview {
    pub fn finish(self) -> Result<ProjectManifest, Diagnostic> {
        if !self.conflicts.is_empty() {
            return Err(diagnostic(
                "merge",
                "unresolved conflicts; edit inputs and preview again",
            ));
        }
        ProjectManifest::parse(&self.candidate.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MigrationPreview {
    pub from: u32,
    pub to: u32,
    pub supported: bool,
    pub changes: Vec<String>,
    pub original: Value,
}
/// V1 is the first schema. No fictional older migrations and no coercion of future versions.
pub fn preview_migration(original: Value, to: u32) -> Result<MigrationPreview, Diagnostic> {
    let from = original
        .get("schema_version")
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| diagnostic("schema_version", "an explicit integer version is required"))?;
    let supported = from == SCHEMA_VERSION && to == SCHEMA_VERSION;
    if supported {
        ProjectManifest::parse(&original.to_string())?;
    }
    Ok(MigrationPreview {
        from,
        to,
        supported,
        changes: if supported {
            vec![]
        } else {
            vec!["No registered migration; original retained without changes".into()]
        },
        original,
    })
}
