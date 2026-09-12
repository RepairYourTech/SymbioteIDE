//! Dispatch preparation: compose the merged primitives into one durable,
//! explainable dispatch record. A `DispatchPreparation` binds the scheduler
//! projection, the routed Role, the task's lease binding, the stream's
//! reserved worktree identity, and the provider registration that the
//! dispatch's runtime profile names and the registry validates — or the
//! typed reason it could not. Compilation into a live
//! `Dispatch` + `Start` transition happens separately in the Host; this layer
//! is the durable, journaled record of the composition and its refusals.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PREPARATION_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreparationOutcome {
    /// Every composition step succeeded; the referenced ids identify the
    /// durable artifacts a Start transition may consume.
    Ready,
    /// One or more composition steps refused, with the recorded reasons.
    Refused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompositionStep {
    Scheduling {
        schedulable: bool,
    },
    Routing {
        resolved: Option<RoleId>,
    },
    Lease {
        held: bool,
    },
    Worktree {
        declared: bool,
    },
    Provider {
        validated: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        refusal: Option<ProviderRefusal>,
    },
}

/// Why the provider registration a dispatch profile names could not be
/// validated. Registry-owned facts only: credential material and verified
/// endpoint locality are execution-time Host concerns and never appear here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRefusal {
    /// No workforce binding resolved for the task's Role, so there is no
    /// runtime profile to validate.
    UnresolvedProfile,
    /// The profile names a connection the registry does not hold.
    MissingConnection,
    /// The profile names an entitlement the registry does not hold.
    MissingEntitlement,
    /// The profile names a model descriptor the registry does not hold.
    MissingModel,
    /// The stored descriptor declares an unsupported provider contract version.
    UnsupportedVersion,
    /// The stored descriptor's own token bounds are invalid.
    InvalidDescriptor,
    /// Profile, connection, entitlement and model identities disagree.
    BindingMismatch,
    /// The entitlement is expired at preparation time.
    ExpiredEntitlement,
    /// The authentication/billing combination is not supported.
    UnsupportedAuthenticationBilling,
}

impl ProviderRefusal {
    /// The refusal's stable wire name — byte-identical to the serialized form
    /// a preparation record carries, for the boundaries that must name a
    /// refusal outside a record (the execution-time refusal message).
    /// A contract test pins it against serde so the two can never drift.
    pub fn name(self) -> &'static str {
        match self {
            Self::UnresolvedProfile => "unresolved_profile",
            Self::MissingConnection => "missing_connection",
            Self::MissingEntitlement => "missing_entitlement",
            Self::MissingModel => "missing_model",
            Self::UnsupportedVersion => "unsupported_version",
            Self::InvalidDescriptor => "invalid_descriptor",
            Self::BindingMismatch => "binding_mismatch",
            Self::ExpiredEntitlement => "expired_entitlement",
            Self::UnsupportedAuthenticationBilling => "unsupported_authentication_billing",
        }
    }
}

/// The provider registration resolved for a task's dispatch profile, and
/// whether the registry proved it usable. The identities are the profile's
/// declared ones, so a refused preparation still names what it refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderResolution {
    pub connection: ProviderConnectionId,
    pub model: ModelId,
    /// Present exactly when the registration did not validate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<ProviderRefusal>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DispatchPreparation {
    pub version: u32,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    pub stream_id: ChangeStreamId,
    pub outcome: PreparationOutcome,
    pub steps: Vec<CompositionStep>,
    /// The routed Role, present only when routing resolved.
    pub role_id: Option<RoleId>,
    /// The lease binding (dispatch + fencing token) when a held lease exists.
    pub lease_dispatch_id: Option<DispatchId>,
    pub fencing_token: Option<u64>,
    /// The stream's reserved worktree identity and branch.
    pub worktree_id: Option<WorktreeId>,
    pub branch: Option<String>,
    /// The provider identities the task's dispatch profile names. Present
    /// whenever a profile resolved, even when its registration was refused.
    pub provider_connection: Option<ProviderConnectionId>,
    pub model_id: Option<ModelId>,
    pub compiled_at: Timestamp,
}

impl DispatchPreparation {
    /// Pure aggregation of already-resolved authoritative inputs. The Host
    /// resolves everything from storage; no field is client-supplied.
    #[allow(clippy::too_many_arguments)]
    pub fn compose(
        task: &Task,
        routed_role: Option<RoleId>,
        lease: Option<(&DispatchId, u64)>,
        stream: &ChangeStream,
        provider: Option<ProviderResolution>,
        at: Timestamp,
    ) -> Self {
        let mut steps = Vec::new();
        // Scheduling: the task must be Ready (this is the projection's
        // schedulable core condition; dependency/stream/lease detail is
        // recorded by the scheduler slice).
        let schedulable = task.state() == &TaskState::Ready;
        steps.push(CompositionStep::Scheduling { schedulable });
        steps.push(CompositionStep::Routing {
            resolved: routed_role.clone(),
        });
        let lease_dispatch = lease.map(|(dispatch, token)| (dispatch.clone(), token));
        // Informational: a lease may already be held (renewal); first-time
        // starts acquire it transactionally during the Start transition.
        steps.push(CompositionStep::Lease {
            held: lease.is_some(),
        });
        // The stream row declares the worktree identity; filesystem
        // reservation is issue 211's separate concern and is NOT checked here.
        steps.push(CompositionStep::Worktree { declared: true });
        // The provider step is earned by the registry validating the
        // profile's stored connection/entitlement/model registration, not by
        // the presence of a profile. A refused registration is recorded with
        // its reason and refuses the composition.
        let refusal = match &provider {
            None => Some(ProviderRefusal::UnresolvedProfile),
            Some(resolution) => resolution.refusal,
        };
        steps.push(CompositionStep::Provider {
            validated: refusal.is_none(),
            refusal,
        });
        let ready = schedulable && routed_role.is_some() && refusal.is_none();
        Self {
            version: PREPARATION_VERSION,
            project_id: task.project_id().clone(),
            task_id: task.id().clone(),
            stream_id: task.stream_id().clone(),
            outcome: if ready {
                PreparationOutcome::Ready
            } else {
                PreparationOutcome::Refused
            },
            steps,
            role_id: routed_role,
            lease_dispatch_id: lease_dispatch.as_ref().map(|(d, _)| d.clone()),
            fencing_token: lease.map(|(_, token)| token),
            worktree_id: Some(stream.worktree.clone()),
            branch: Some(stream.branch.clone()),
            provider_connection: provider
                .as_ref()
                .map(|resolution| resolution.connection.clone()),
            model_id: provider.map(|resolution| resolution.model),
            compiled_at: at,
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.version != PREPARATION_VERSION || self.steps.is_empty() || self.steps.len() > 8 {
            return Err(DomainError::InvalidStream);
        }
        for step in &self.steps {
            // The two provider fields must agree: a refusal is recorded
            // exactly when the step did not validate.
            if let CompositionStep::Provider { validated, refusal } = step {
                if *validated != refusal.is_none() {
                    return Err(DomainError::InvalidStream);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderRefusal;

    /// `name()` is the refusal's identity on the wire everywhere it cannot be
    /// a serialized field. Pin it to the serde rename so the two cannot drift.
    #[test]
    fn provider_refusal_names_match_their_serialized_form() {
        for refusal in [
            ProviderRefusal::UnresolvedProfile,
            ProviderRefusal::MissingConnection,
            ProviderRefusal::MissingEntitlement,
            ProviderRefusal::MissingModel,
            ProviderRefusal::UnsupportedVersion,
            ProviderRefusal::InvalidDescriptor,
            ProviderRefusal::BindingMismatch,
            ProviderRefusal::ExpiredEntitlement,
            ProviderRefusal::UnsupportedAuthenticationBilling,
        ] {
            let serialized = serde_json::to_value(refusal).unwrap();
            assert_eq!(
                refusal.name(),
                serialized.as_str().unwrap(),
                "{refusal:?} name drifts from its serialized form"
            );
        }
    }
}
