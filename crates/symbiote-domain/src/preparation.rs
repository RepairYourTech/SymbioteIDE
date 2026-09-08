//! Dispatch preparation: compose the merged primitives into one durable,
//! explainable dispatch record. A `DispatchPreparation` binds the scheduler
//! projection, the routed Role, the task's lease binding, the stream's
//! reserved worktree identity, and the validated provider binding that the
//! dispatch's runtime profile resolves to. Compilation into a live
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
    Scheduling { schedulable: bool },
    Routing { resolved: Option<RoleId> },
    Lease { held: bool },
    Worktree { declared: bool },
    Provider { validated: bool },
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
    /// The provider binding resolved for the task's dispatch profile.
    pub provider_connection: Option<ProviderConnectionId>,
    pub model_id: Option<ModelId>,
    pub compiled_at: Timestamp,
}

impl DispatchPreparation {
    /// Pure aggregation of already-resolved authoritative inputs. The Host
    /// resolves everything from storage; no field is client-supplied.
    #[allow(clippy::too_many_arguments)]
    /// Pure aggregation of already-resolved authoritative inputs. The Host
    /// resolves everything from storage; no field is client-supplied.
    #[allow(clippy::too_many_arguments)]
    pub fn compose(
        task: &Task,
        routed_role: Option<RoleId>,
        lease: Option<(&DispatchId, u64)>,
        stream: &ChangeStream,
        provider: Option<(&ProviderConnectionId, &ModelId)>,
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
        steps.push(CompositionStep::Lease {
            held: lease.is_some(),
        });
        // The stream row declares the worktree identity; filesystem
        // reservation is issue 211's separate concern and is NOT checked here.
        steps.push(CompositionStep::Worktree { declared: true });
        let provider_resolved = provider.is_some();
        steps.push(CompositionStep::Provider {
            validated: provider_resolved,
        });
        let ready = schedulable && routed_role.is_some() && lease.is_some() && provider_resolved;
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
            provider_connection: provider.map(|(connection, _)| connection.clone()),
            model_id: provider.map(|(_, model)| model.clone()),
            compiled_at: at,
        }
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.version != PREPARATION_VERSION || self.steps.is_empty() || self.steps.len() > 8 {
            return Err(DomainError::InvalidStream);
        }
        Ok(())
    }
}
