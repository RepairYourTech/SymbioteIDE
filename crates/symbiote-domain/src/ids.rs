use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

/// IDs are assigned by the future authority; this crate deliberately does not derive
/// identities from branch names, provider names, GitHub numbers, or network addresses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidId;

impl fmt::Display for InvalidId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("identity must be 1..=128 ASCII letters, digits, underscores or hyphens")
    }
}
impl std::error::Error for InvalidId {}

macro_rules! ids {
    ($($name:ident),+ $(,)?) => {$(
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl JsonSchema for $name {
            fn schema_name() -> std::borrow::Cow<'static, str> { stringify!($name).into() }
            fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
                schemars::json_schema!({ "type": "string", "minLength": 1, "maxLength": 128, "pattern": "^[A-Za-z0-9_-]+$" })
            }
        }
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidId> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') {
                    return Err(InvalidId);
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = InvalidId;
            fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(value) }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self { value.0 }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }
    )+};
}

ids!(
    UserId,
    HostId,
    FabricId,
    DeviceId,
    ClientId,
    ControllerSessionId,
    ProjectId,
    RootId,
    EnvironmentId,
    RoleId,
    BindingId,
    RuntimeProfileId,
    HarnessDriverId,
    InstallationId,
    ModelId,
    AgentRuntimeAdapterId,
    InferenceProviderAdapterId,
    ProviderConnectionId,
    CredentialReferenceId,
    BillingEntitlementId,
    TaskId,
    DispatchId,
    RuntimeContractId,
    ChangeStreamId,
    SessionId,
    ChatId,
    WorktreeId,
    ArtifactId,
    EvidenceId,
    VerificationRunId,
    RequestId,
    ObjectiveId,
    CapabilityId,
    KnowledgeId,
    RequirementId,
    DecisionId,
    AmbiguityId,
    DiagnosticId,
    AnnotationId,
    PluginId,
    ReleaseTargetId,
    GoalRunId,
    NativeExecutionId,
    ExecutionEpisodeId,
    LearnedMethodId,
    ExperimentId,
    ProtocolId,
    RoleContractId,
    TaskContractId,
    ContextBundleId,
    CommandId,
    SecretLeaseId,
    ProjectionId,
    PairingId
);

/// Immutable source identity; accepts SHA-1 and SHA-256 Git object formats.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CommitSha(String);

impl JsonSchema for CommitSha {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "CommitSha".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "pattern": "^([A-Fa-f0-9]{40}|[A-Fa-f0-9]{64})$" })
    }
}

impl CommitSha {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if !matches!(value.len(), 40 | 64) || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("commit must be a full 40- or 64-character hexadecimal object id");
        }
        Ok(Self(value.to_ascii_lowercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for CommitSha {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<CommitSha> for String {
    fn from(value: CommitSha) -> Self {
        value.0
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct Revision(pub u64);

/// UTC milliseconds from Unix epoch, assigned by the authority, not a worker clock.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct Timestamp(pub u64);
