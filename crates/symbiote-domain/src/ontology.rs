//! The canonical vocabulary: one owner for every noun (#36).
//!
//! #36's first acceptance criterion is that every constitution noun maps to
//! exactly one canonical entity or value object. That is a claim about this
//! crate's own inventory, so it is held as data and checked rather than asserted
//! in prose. Each noun names the identity that identifies it and the one type
//! that owns its behavior, and the rules in `problems` tie both directions to
//! what the crate actually declares: the identity side to the identity catalogue
//! the crate compiles, the owner side to the schema it publishes.
//!
//! The tie is a round trip, so a record cannot arrive ownerless and unnoticed.
//! Every name the artifact publishes is either a canonical type or a definition
//! one of them references; every canonical type is owned by exactly one noun, or
//! is an identity-only type excused with its reason (an identity is named by the
//! noun that carries it, and is not a noun itself); every record the envelope
//! carries is a canonical type whose noun is an entity, since a value object is
//! identified only through its carrier and cannot stand as a record of its own;
//! and every entity noun's owner is a record the envelope carries. Those two last
//! rules together are the bijection: the records and the entity nouns are the
//! same set, so a new type cannot be added on one side only.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Whether a noun's canonical representation carries identity of its own or is a
/// value object identified only as part of something else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Carries identity of its own, so it can be carried as a record: a revision,
    /// a disposition and a lifecycle state where the noun has states.
    Entity,
    /// No identity of its own — immutable, or identified only through the record
    /// that carries it — so it cannot stand as a record the envelope carries.
    ValueObject,
}

/// One noun and its single canonical owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noun {
    /// The noun as #36 and the product constitution name it.
    pub noun: &'static str,
    /// The identity that identifies it, when it has one of its own.
    pub identity: Option<&'static str>,
    /// The one type that owns it.
    pub owner: &'static str,
    pub shape: Shape,
    /// The lifecycle state type, when the noun has one.
    pub lifecycle: Option<&'static str>,
}

/// A type this ontology publishes, with the schema it publishes for it.
pub struct Canonical {
    pub name: &'static str,
    pub schema: fn() -> schemars::Schema,
}

macro_rules! canonical {
    ($($t:ty),+ $(,)?) => {
        /// Every type this ontology publishes, declared once. The schema below is
        /// generated from this list, so the published artifact and the inventory
        /// cannot disagree about what exists.
        pub const CANONICAL: &[Canonical] = &[$(Canonical {
            name: stringify!($t),
            schema: || schemars::schema_for!($t),
        }),+];
    };
}

canonical!(
    // Principals, Fabric and clients
    crate::entities::User,
    crate::entities::Fabric,
    crate::entities::Device,
    crate::entities::Client,
    crate::entities::ControllerSession,
    crate::entities::Pairing,
    crate::entities::Environment,
    crate::entities::HarnessDriver,
    crate::entities::Installation,
    crate::entities::Model,
    crate::entities::Request,
    crate::entities::Objective,
    crate::entities::Capability,
    crate::entities::Chat,
    crate::entities::Worktree,
    crate::entities::Plan,
    crate::entities::Milestone,
    crate::entities::Checkpoint,
    crate::entities::FollowUp,
    crate::entities::Requirement,
    crate::entities::Decision,
    crate::entities::Ambiguity,
    crate::entities::Diagnostic,
    crate::entities::Annotation,
    crate::entities::Plugin,
    crate::entities::ReleaseTarget,
    crate::entities::VerificationRun,
    crate::entities::SecretLease,
    crate::entities::Projection,
    crate::entities::ConfigProjection,
    // The records this crate already carried, named at the crate root so the
    // list does not depend on which module happens to re-export them.
    crate::HostId,
    crate::AgentRuntimeAdapterId,
    crate::InferenceProviderAdapterId,
    crate::CredentialReference,
    crate::CredentialReferenceId,
    crate::ContextBundleId,
    crate::CommandId,
    crate::Project,
    crate::Root,
    crate::Role,
    crate::Host,
    crate::RuntimeProfile,
    crate::WorkforceBinding,
    crate::WorkforceRuntimeContract,
    crate::ProviderConnection,
    crate::BillingEntitlement,
    crate::Task,
    crate::Dispatch,
    crate::ChangeStream,
    crate::Session,
    crate::Artifact,
    crate::VerificationEvidence,
    crate::KnowledgeClaim,
    crate::GoalRun,
    crate::NativeExecution,
    crate::ExecutionEpisode,
    crate::LearnedMethod,
    crate::Experiment,
    crate::ExternalReference,
    crate::VersionedProtocol,
    crate::VersionedRoleContract,
    crate::VersionedTaskContract,
    crate::TaskDependencyEdge,
);

/// The committed ontology artifact, relative to the workspace root. `--write`
/// regenerates it and `--check` reports drift, so the published inventory cannot
/// silently diverge from the types this crate compiles.
pub const ONTOLOGY_SCHEMA_PATH: &str = "docs/contracts/domain-ontology.v1.schema.json";

/// The workspace root, from this crate's manifest directory.
pub fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}

/// The last path segment of a type name, so the vocabulary and the artifact can
/// name a type without the module it happens to live in.
fn short(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

/// The name the vocabulary and the published artifact use for one canonical type.
pub fn canonical_name(entry: &Canonical) -> &'static str {
    short(entry.name)
}

/// Every canonical type name, in declaration order.
pub fn canonical_names() -> Vec<&'static str> {
    CANONICAL.iter().map(canonical_name).collect()
}

/// The machine-readable ontology: one definition per named type, an index from
/// the canonical names to those definitions, and the wire records the envelope
/// can carry. Definitions are held once, so the artifact does not repeat every
/// dependency inside every entity.
pub fn ontology_schema() -> serde_json::Value {
    let mut definitions = serde_json::Map::new();
    let mut entities = serde_json::Map::new();
    for entry in CANONICAL {
        let mut schema = serde_json::to_value((entry.schema)()).expect("schema serialization");
        if let Some(nested) = schema.get("$defs").and_then(|value| value.as_object()) {
            for (name, definition) in nested {
                definitions
                    .entry(name.clone())
                    .or_insert_with(|| definition.clone());
            }
        }
        if let Some(object) = schema.as_object_mut() {
            object.remove("$defs");
            object.remove("$schema");
        }
        let name = canonical_name(entry).to_string();
        definitions.insert(name.clone(), schema);
        entities.insert(
            name.clone(),
            serde_json::json!({ "$ref": format!("#/$defs/{name}") }),
        );
    }
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://symbiote.dev/schemas/symbiote.domain-ontology.v1.schema.json",
        "schema_version": crate::SCHEMA_VERSION,
        "description": "Every canonical domain type (#36), one definition each, plus the records DomainEnvelope carries.",
        "$defs": definitions,
        "entities": entities,
        "envelope_records": envelope_record_names(),
    })
}

/// The committed encoding: pretty JSON with a trailing newline, matching how this
/// repository's other generated documents are stored so a diff is readable.
pub fn ontology_text() -> String {
    format!(
        "{}\n",
        serde_json::to_string_pretty(&ontology_schema()).expect("schema serialization")
    )
}

/// The record types `DomainRecord` can carry, read from the envelope's own
/// schema rather than restated, so an entity that is declared but not
/// transportable is visible to [`problems`].
pub fn envelope_record_names() -> BTreeSet<String> {
    let schema = serde_json::to_value(schemars::schema_for!(crate::DomainRecord))
        .expect("schema serialization");
    schema
        .get("oneOf")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|variant| variant.get("properties")?.get("record")?.get("$ref"))
        .filter_map(|reference| reference.as_str())
        .filter_map(|reference| reference.rsplit('/').next())
        .map(str::to_owned)
        .collect()
}

/// The names the vocabulary's owners and lifecycles must resolve against, read
/// from a published ontology document rather than from the inventory above, so a
/// claim is checked against the artifact that was committed. Both the definitions
/// and the index count: an owner may be a canonical type or a type it depends on.
pub fn published_names(schema: &serde_json::Value) -> BTreeSet<String> {
    let keys = |field: &str| -> BTreeSet<String> {
        schema
            .get(field)
            .and_then(|value| value.as_object())
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    };
    keys("$defs").union(&keys("entities")).cloned().collect()
}

/// The canonical type names the document indexes — the names a noun must own.
pub fn indexed_names(schema: &serde_json::Value) -> BTreeSet<String> {
    schema
        .get("entities")
        .and_then(|value| value.as_object())
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

/// The names a published document's definitions refer to. A definition that is
/// not a canonical type is one of these: a dependency of the shape of a type that
/// is, rather than a noun of its own.
pub fn referenced_names(schema: &serde_json::Value) -> BTreeSet<String> {
    fn walk(value: &serde_json::Value, into: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Object(object) => {
                for (key, value) in object {
                    if key == "$ref" {
                        if let Some(name) = value.as_str().and_then(|r| r.rsplit('/').next()) {
                            into.insert(name.to_owned());
                        }
                    } else {
                        walk(value, into);
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|item| walk(item, into)),
            _ => {}
        }
    }
    let mut names = BTreeSet::new();
    for field in ["$defs", "entities"] {
        if let Some(value) = schema.get(field) {
            walk(value, &mut names);
        }
    }
    names
}

/// The records a published document says the envelope carries.
pub fn recorded_names(schema: &serde_json::Value) -> BTreeSet<String> {
    schema
        .get("envelope_records")
        .and_then(|value| value.as_array())
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A criterion this layer cannot check, with the work that owns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outstanding {
    /// The acceptance criterion or required scope item.
    pub criterion: &'static str,
    /// The canonical issue that owns it.
    pub issue: u64,
    /// What is missing here, and why this layer cannot supply it.
    pub why: &'static str,
}

const fn noun(
    noun: &'static str,
    identity: &'static str,
    owner: &'static str,
    shape: Shape,
    lifecycle: Option<&'static str>,
) -> Noun {
    Noun {
        noun,
        identity: Some(identity),
        owner,
        shape,
        lifecycle,
    }
}

const fn value(noun: &'static str, owner: &'static str) -> Noun {
    Noun {
        noun,
        identity: None,
        owner,
        shape: Shape::ValueObject,
        lifecycle: None,
    }
}

/// The canonical vocabulary: one entry per noun, one identity per entry, and one
/// owner per type. `tests/ontology.rs` holds all three to be total.
pub const VOCABULARY: &[Noun] = &[
    // Principals, Fabric and clients
    noun("user", "UserId", "User", Shape::Entity, None),
    noun("host", "HostId", "Host", Shape::Entity, None),
    noun(
        "fabric",
        "FabricId",
        "Fabric",
        Shape::Entity,
        Some("FabricState"),
    ),
    noun(
        "device",
        "DeviceId",
        "Device",
        Shape::Entity,
        Some("DeviceState"),
    ),
    noun(
        "client",
        "ClientId",
        "Client",
        Shape::Entity,
        Some("ClientState"),
    ),
    noun(
        "controller session",
        "ControllerSessionId",
        "ControllerSession",
        Shape::Entity,
        Some("ControllerSessionState"),
    ),
    noun(
        "pairing",
        "PairingId",
        "Pairing",
        Shape::Entity,
        Some("PairingState"),
    ),
    // Project structure, staffing and runtime inventory
    noun("project", "ProjectId", "Project", Shape::Entity, None),
    noun("root/repository", "RootId", "Root", Shape::Entity, None),
    noun("role", "RoleId", "Role", Shape::Entity, None),
    noun(
        "workforce binding",
        "BindingId",
        "WorkforceBinding",
        Shape::Entity,
        None,
    ),
    noun(
        "workforce runtime contract",
        "RuntimeContractId",
        "WorkforceRuntimeContract",
        Shape::Entity,
        None,
    ),
    noun(
        "runtime profile/account identity",
        "RuntimeProfileId",
        "RuntimeProfile",
        Shape::Entity,
        None,
    ),
    noun(
        "environment",
        "EnvironmentId",
        "Environment",
        Shape::Entity,
        Some("EnvironmentState"),
    ),
    noun(
        "harness driver",
        "HarnessDriverId",
        "HarnessDriver",
        Shape::Entity,
        Some("DriverState"),
    ),
    noun(
        "harness installation",
        "InstallationId",
        "Installation",
        Shape::Entity,
        None,
    ),
    noun(
        "model",
        "ModelId",
        "Model",
        Shape::Entity,
        Some("ModelState"),
    ),
    noun(
        "agent runtime adapter",
        "AgentRuntimeAdapterId",
        "AgentRuntimeAdapterId",
        Shape::ValueObject,
        None,
    ),
    noun(
        "inference provider adapter",
        "InferenceProviderAdapterId",
        "InferenceProviderAdapterId",
        Shape::ValueObject,
        None,
    ),
    noun(
        "provider connection",
        "ProviderConnectionId",
        "ProviderConnection",
        Shape::Entity,
        None,
    ),
    noun(
        "credential reference",
        "CredentialReferenceId",
        "CredentialReference",
        Shape::Entity,
        None,
    ),
    noun(
        "billing entitlement",
        "BillingEntitlementId",
        "BillingEntitlement",
        Shape::Entity,
        None,
    ),
    // Work, mutation and completion
    noun("task", "TaskId", "Task", Shape::Entity, Some("TaskState")),
    noun(
        "request",
        "RequestId",
        "Request",
        Shape::Entity,
        Some("RequestState"),
    ),
    noun(
        "objective",
        "ObjectiveId",
        "Objective",
        Shape::Entity,
        Some("ObjectiveState"),
    ),
    noun(
        "capability",
        "CapabilityId",
        "Capability",
        Shape::Entity,
        Some("CapabilityState"),
    ),
    noun("dispatch", "DispatchId", "Dispatch", Shape::Entity, None),
    noun(
        "change stream",
        "ChangeStreamId",
        "ChangeStream",
        Shape::Entity,
        Some("StreamState"),
    ),
    noun("session", "SessionId", "Session", Shape::Entity, None),
    noun("chat", "ChatId", "Chat", Shape::Entity, Some("ChatState")),
    noun(
        "worktree",
        "WorktreeId",
        "Worktree",
        Shape::Entity,
        Some("WorktreeState"),
    ),
    noun("artifact", "ArtifactId", "Artifact", Shape::Entity, None),
    noun(
        "verification run",
        "VerificationRunId",
        "VerificationRun",
        Shape::Entity,
        Some("VerificationRunState"),
    ),
    noun(
        "verification evidence",
        "EvidenceId",
        "VerificationEvidence",
        Shape::Entity,
        None,
    ),
    // Knowledge, documentation, requirements and records of record
    noun(
        "developer knowledge claim",
        "KnowledgeId",
        "KnowledgeClaim",
        Shape::Entity,
        None,
    ),
    noun(
        "knowledge projection",
        "ProjectionId",
        "Projection",
        Shape::Entity,
        Some("ProjectionState"),
    ),
    noun(
        "runtime config projection",
        "ConfigProjectionId",
        "ConfigProjection",
        Shape::Entity,
        Some("ConfigState"),
    ),
    noun(
        "requirement",
        "RequirementId",
        "Requirement",
        Shape::Entity,
        Some("RequirementState"),
    ),
    noun(
        "decision",
        "DecisionId",
        "Decision",
        Shape::Entity,
        Some("DecisionState"),
    ),
    noun(
        "ambiguity",
        "AmbiguityId",
        "Ambiguity",
        Shape::Entity,
        Some("AmbiguityState"),
    ),
    noun(
        "diagnostic",
        "DiagnosticId",
        "Diagnostic",
        Shape::Entity,
        Some("DiagnosticState"),
    ),
    noun(
        "annotation",
        "AnnotationId",
        "Annotation",
        Shape::Entity,
        Some("AnnotationState"),
    ),
    // Planning hierarchy
    noun("plan", "PlanId", "Plan", Shape::Entity, Some("PlanState")),
    noun(
        "milestone",
        "MilestoneId",
        "Milestone",
        Shape::Entity,
        Some("MilestoneState"),
    ),
    noun(
        "checkpoint",
        "CheckpointId",
        "Checkpoint",
        Shape::Entity,
        Some("CheckpointState"),
    ),
    noun(
        "follow-up",
        "FollowUpId",
        "FollowUp",
        Shape::Entity,
        Some("FollowUpState"),
    ),
    // Extension, release, secrets and delegation
    noun(
        "plugin",
        "PluginId",
        "Plugin",
        Shape::Entity,
        Some("PluginState"),
    ),
    noun(
        "release target",
        "ReleaseTargetId",
        "ReleaseTarget",
        Shape::Entity,
        Some("ReleaseTargetState"),
    ),
    noun(
        "secret lease",
        "SecretLeaseId",
        "SecretLease",
        Shape::Entity,
        Some("SecretLeaseState"),
    ),
    noun("goal run", "GoalRunId", "GoalRun", Shape::Entity, None),
    noun(
        "native execution/delegation",
        "NativeExecutionId",
        "NativeExecution",
        Shape::Entity,
        None,
    ),
    noun(
        "execution episode",
        "ExecutionEpisodeId",
        "ExecutionEpisode",
        Shape::Entity,
        None,
    ),
    noun(
        "learned method",
        "LearnedMethodId",
        "LearnedMethod",
        Shape::Entity,
        None,
    ),
    noun(
        "experiment",
        "ExperimentId",
        "Experiment",
        Shape::Entity,
        None,
    ),
    // Revision-bearing contracts and pure identities
    noun(
        "workforce protocol",
        "ProtocolId",
        "VersionedProtocol",
        Shape::ValueObject,
        None,
    ),
    noun(
        "role operating contract",
        "RoleContractId",
        "VersionedRoleContract",
        Shape::ValueObject,
        None,
    ),
    noun(
        "task contract",
        "TaskContractId",
        "VersionedTaskContract",
        Shape::ValueObject,
        None,
    ),
    noun(
        "context bundle",
        "ContextBundleId",
        "ContextBundleId",
        Shape::ValueObject,
        None,
    ),
    noun(
        "mutation command",
        "CommandId",
        "CommandId",
        Shape::ValueObject,
        None,
    ),
    // Nouns with no identity of their own
    value("dependency", "TaskDependencyEdge"),
    value("external reference", "ExternalReference"),
];

/// Whether a list of excused names holds this one.
fn excused(list: &[(&str, &str)], name: &str) -> bool {
    list.iter().any(|(excused, _)| *excused == name)
}

/// The canonical types no noun owns, each with its reason. Only an identity-only
/// type is admissible: it is published because the noun that carries it names it
/// as its identity, and that noun is already held to exactly one identity by the
/// rules in `problems`. Every entry is checked in both directions — one whose
/// name is gone fails, one whose name owns a noun fails, one that is the identity
/// of no noun fails, and one that states no reason fails — so this cannot become
/// a place to park a type that should have a noun of its own.
pub const IDENTITY_ONLY: &[(&str, &str)] = &[
    (
        "HostId",
        "the identity the `host` noun names: a host is identified by it, so it is an identity rather than a noun of its own",
    ),
    (
        "CredentialReferenceId",
        "the identity the `credential reference` noun names: a credential reference is identified by it, so it is an identity rather than a noun of its own",
    ),
];

/// The criteria of #36 that this layer cannot check, each with the canonical
/// issue that owns it. Nothing here is claimed as delivered.
pub const OUTSTANDING: &[Outstanding] = &[
    Outstanding {
        criterion: "Schema evolution includes forward/backward migration rules.",
        issue: 43,
        why: "The version gate and the breaking-change rule are enforced here, but there is no prior persisted format to convert from until the transactional control-plane store lands.",
    },
    Outstanding {
        criterion: "Desktop, Host, mobile, plugin, adapter and documentation contract tests serialize the same entities.",
        issue: 182,
        why: "Only a Rust consumer exists. The client SDK, the mobile clients, the plugin contracts and the adapter SDK are the consumers that must serialize these entities too.",
    },
    Outstanding {
        criterion: "One local machine and a heterogeneous multi-Host Fabric use the same semantics.",
        issue: 272,
        why: "Fabric and Host identity, membership and states are modelled here; placement, connectivity and authority across several Hosts are not implemented in a domain contract.",
    },
    Outstanding {
        criterion: "The same client request has external-only, native-only and hybrid conformance paths without changing canonical authority.",
        issue: 464,
        why: "The shared canonical contract is exercised here; real native, external and hybrid execution integrations are owned by the runtime issues.",
    },
    Outstanding {
        criterion: "No metric/prompt/native completion/learning candidate can weaken a required security, documentation, impact, review or delivery gate.",
        issue: 449,
        why: "This layer checks the part that is structural: a dispatch whose control enforcement is weaker than the binding requires is refused, and a learned method carries no field that could grant, relax or exempt a gate. That a runtime, prompt or metric cannot weaken a gate in execution is enforced by the runtime and learning owners, not by a schema.",
    },
];

/// The rules that make the vocabulary true. Returns one line per violation, so
/// an empty result is the only passing state. Everything except the vocabulary
/// itself is read from the document, so a caller cannot compare the vocabulary
/// with a set it assembled and call that agreement.
pub fn problems(
    vocabulary: &[Noun],
    outstanding: &[Outstanding],
    identities: &[&str],
    schema: &serde_json::Value,
) -> Vec<String> {
    let published = published_names(schema);
    let indexed = indexed_names(schema);
    let referenced = referenced_names(schema);
    let records = recorded_names(schema);
    let mut problems = Vec::new();
    let mut nouns: BTreeMap<&str, usize> = BTreeMap::new();
    let mut owners: BTreeMap<&str, usize> = BTreeMap::new();
    let mut claimed: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in vocabulary {
        *nouns.entry(entry.noun).or_default() += 1;
        *owners.entry(entry.owner).or_default() += 1;
        let Some(identity) = entry.identity else {
            if entry.shape == Shape::Entity {
                problems.push(format!(
                    "{}: an entity must have an identity of its own",
                    entry.noun
                ));
            }
            continue;
        };
        *claimed.entry(identity).or_default() += 1;
        if !identities.contains(&identity) {
            problems.push(format!(
                "{}: identity {identity} is not declared by this crate",
                entry.noun
            ));
        }
    }
    for (noun, count) in &nouns {
        if *count > 1 {
            problems.push(format!("noun {noun} maps to {count} owners"));
        }
    }
    for (owner, count) in &owners {
        if *count > 1 {
            problems.push(format!("type {owner} owns {count} nouns"));
        }
    }
    for (identity, count) in &claimed {
        if *count > 1 {
            problems.push(format!("identity {identity} identifies {count} nouns"));
        }
    }
    for identity in identities {
        if !claimed.contains_key(identity) {
            problems.push(format!("identity {identity} has no owning noun"));
        }
    }
    for entry in vocabulary {
        if !published.contains(entry.owner) {
            problems.push(format!(
                "{}: owner {} is not in the published schema",
                entry.noun, entry.owner
            ));
        }
        if let Some(lifecycle) = entry.lifecycle {
            if !published.contains(lifecycle) {
                problems.push(format!(
                    "{}: lifecycle {lifecycle} is not in the published schema",
                    entry.noun
                ));
            }
        }
    }
    // Every entity noun's owner is a record, and every record's noun is an
    // entity. Together these are the round trip between the inventory and the
    // transport: a record with no noun, and a noun with no record, both fail.
    for entry in vocabulary {
        if entry.shape == Shape::Entity && !records.contains(entry.owner) {
            problems.push(format!(
                "{}: entity {} is not a record the envelope can carry",
                entry.noun, entry.owner
            ));
        }
    }
    for name in &records {
        if !indexed.contains(name) {
            problems.push(format!(
                "the envelope carries {name}, which is not a canonical type this crate publishes"
            ));
            continue;
        }
        match vocabulary.iter().find(|entry| entry.owner == name) {
            None => problems.push(format!("the record {name} is owned by no noun")),
            Some(entry) if entry.shape != Shape::Entity => problems.push(format!(
                "the record {name} is not an entity: its noun {} is a value object",
                entry.noun
            )),
            Some(_) => {}
        }
    }
    // Every canonical type is indexed under its own name, nothing else is, and
    // every indexed name is owned by exactly one noun or excused as identity-only.
    for entry in CANONICAL {
        let name = canonical_name(entry);
        if !indexed.contains(name) {
            problems.push(format!(
                "canonical type {name} is not indexed in the published artifact"
            ));
        }
    }
    for name in &indexed {
        if !canonical_names().contains(&name.as_str()) {
            problems.push(format!(
                "the published artifact indexes {name}, which is not a canonical type this crate publishes"
            ));
            continue;
        }
        if owners.contains_key(name.as_str()) || excused(IDENTITY_ONLY, name) {
            continue;
        }
        problems.push(format!(
            "canonical type {name} is owned by no noun and is not listed as identity-only"
        ));
    }
    for (name, why) in IDENTITY_ONLY {
        if !indexed.contains(*name) {
            problems.push(format!(
                "the identity-only entry {name} is not a canonical type"
            ));
        }
        if owners.contains_key(name) {
            let noun = vocabulary
                .iter()
                .find(|noun| noun.owner == *name)
                .map_or("", |noun| noun.noun);
            problems.push(format!(
                "the identity-only entry {name} owns the noun {noun}, so it is not identity-only"
            ));
        } else if !claimed.contains_key(name) {
            problems.push(format!(
                "the identity-only entry {name} is the identity of no noun"
            ));
        }
        if why.trim().len() < 40 {
            problems.push(format!("the identity-only entry {name} states no reason"));
        }
    }
    // Every published name is accounted for: a canonical type, or a definition a
    // canonical type refers to. A name that is neither is published by nothing.
    for name in &published {
        if !indexed.contains(name) && !referenced.contains(name) {
            problems.push(format!(
                "published name {name} is neither a canonical type nor referenced by one"
            ));
        }
    }
    for entry in outstanding {
        if entry.issue == 0 {
            problems.push(format!("{}: names no owning issue", entry.criterion));
        }
        if entry.why.trim().is_empty() {
            problems.push(format!("{}: states no reason", entry.criterion));
        }
    }
    problems
}
