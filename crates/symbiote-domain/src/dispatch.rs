use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Historical dispatch contract. Fields have no setters: re-staffing produces a
/// new binding revision and a new Dispatch, never an in-place contract rewrite.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkforceRuntimeContract {
    binding: WorkforceBinding,
    role: Role,
    profile: RuntimeProfile,
    host_id: HostId,
    host_revision: Revision,
    task_id: TaskId,
    root_id: RootId,
    stream_id: ChangeStreamId,
    task_revision: Revision,
    task_contract: VersionedTaskContract,
    gates: std::collections::BTreeSet<Gate>,
    enforcement: BTreeMap<Control, EnforcementClaim>,
    /// The least-privilege access this dispatch operates under: the binding's
    /// authorized access narrowed to exactly this Task's Root. `None` only on
    /// contracts journaled before narrowing existed (they fall back to the
    /// full binding snapshot they were compiled with).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effective_access: Option<AccessSnapshot>,
    /// The resource limits this dispatch was staffed with. They are recorded,
    /// not merely referenced: the Host binds the execution it produces to
    /// exactly these, and a limit it cannot bind refuses that execution.
    /// `None` only on contracts journaled before limits were recorded: they
    /// deserialize rather than break replay, and activation refuses them for
    /// having no bound to apply instead of running unbounded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    limits: Option<ResourceLimits>,
    compiled_at: Timestamp,
}

impl WorkforceRuntimeContract {
    pub fn binding(&self) -> &WorkforceBinding {
        &self.binding
    }
    pub fn enforcement(&self) -> &BTreeMap<Control, EnforcementClaim> {
        &self.enforcement
    }
    /// The narrowed access this dispatch runs under. Contracts compiled
    /// before per-dispatch narrowing fell back to the full binding snapshot.
    pub fn effective_access(&self) -> &AccessSnapshot {
        self.effective_access
            .as_ref()
            .unwrap_or(&self.binding.access)
    }
    pub fn role(&self) -> &Role {
        &self.role
    }
    pub fn profile(&self) -> &RuntimeProfile {
        &self.profile
    }
    pub fn host_id(&self) -> &HostId {
        &self.host_id
    }
    pub fn task_id(&self) -> &TaskId {
        &self.task_id
    }
    pub fn root_id(&self) -> &RootId {
        &self.root_id
    }
    pub fn stream_id(&self) -> &ChangeStreamId {
        &self.stream_id
    }
    pub fn task_contract(&self) -> &VersionedTaskContract {
        &self.task_contract
    }
    pub fn task_revision(&self) -> Revision {
        self.task_revision
    }
    pub fn gates(&self) -> &std::collections::BTreeSet<Gate> {
        &self.gates
    }
    /// The limits this dispatch's execution is bound by, or `None` on a
    /// contract journaled before limits were recorded — which activation
    /// refuses rather than running unbounded.
    pub fn limits(&self) -> Option<&ResourceLimits> {
        self.limits.as_ref()
    }

    pub fn validate_at(&self, now: Timestamp) -> Result<(), DomainError> {
        if now < self.compiled_at {
            return Err(DomainError::InvalidTimestamp);
        }
        if self.binding.role_id != self.role.id
            || self.binding.project_id != self.role.project_id
            || self.binding.access.project_id != self.role.project_id
            || self.binding.profile_id != self.profile.id
            || self.binding.profile_revision != self.profile.revision
        {
            return Err(DomainError::LineageMismatch);
        }
        if !self.profile.eligible_hosts.contains(&self.host_id) {
            return Err(DomainError::IneligibleHost);
        }
        if !self
            .binding
            .access
            .grants
            .contains(&Permission::MutateStream)
            || !self.binding.access.roots.contains(&self.root_id)
        {
            return Err(DomainError::PermissionDenied);
        }
        if self.binding.context.max_input_tokens == 0
            || self.binding.context.reserved_output_tokens == 0
        {
            return Err(DomainError::InvalidContextBudget);
        }
        // A recorded limit is a validated invariant like every other field: a
        // replayed contract can never carry a limit the contract does not
        // allow. A contract recording none is tolerated — it is older state,
        // not a false claim — and the execution boundary refuses it for having
        // no bound to apply.
        if self
            .limits
            .as_ref()
            .is_some_and(|limits| !limits.validate())
        {
            return Err(DomainError::ResourceLimit);
        }
        if self.gates != Gate::required() {
            return Err(DomainError::EvidenceMissing);
        }
        let mut required = self.binding.required_controls.clone();
        required.extend([
            Control::Filesystem,
            Control::Cancellation,
            Control::CompletionAuthority,
        ]);
        for (permission, control) in [
            (Permission::ExecuteProcess, Control::Process),
            (Permission::Network, Control::Network),
            (Permission::UseCredential, Control::Credentials),
        ] {
            if self.binding.access.grants.contains(&permission) {
                required.insert(control);
            }
        }
        if self
            .enforcement
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            != required
        {
            return Err(DomainError::UnsupportedControl);
        }
        for claim in self.enforcement.values() {
            if !matches!(
                claim.strength,
                EnforcementStrength::Native | EnforcementStrength::HostEnforced
            ) || claim.verified_at > self.compiled_at
                || claim.expires_at <= now
            {
                return Err(DomainError::UnsupportedControl);
            }
        }
        // The narrowed scope is a validated invariant, not a convention: a
        // reconstructed contract (journal replay) must carry an
        // effective_access that is exactly this Task's Root under the
        // binding's own authorization — never broader, never another root.
        if let Some(effective) = &self.effective_access {
            let narrowed =
                std::iter::once(self.root_id.clone()).collect::<std::collections::BTreeSet<_>>();
            if effective.project_id != self.binding.access.project_id
                || effective.policy_revision != self.binding.access.policy_revision
                || effective.grants != self.binding.access.grants
                || !self.binding.access.roots.is_superset(&narrowed)
                || effective.roots != narrowed
            {
                return Err(DomainError::LineageMismatch);
            }
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for WorkforceRuntimeContract {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            binding: WorkforceBinding,
            role: Role,
            profile: RuntimeProfile,
            host_id: HostId,
            host_revision: Revision,
            task_id: TaskId,
            root_id: RootId,
            stream_id: ChangeStreamId,
            task_revision: Revision,
            task_contract: VersionedTaskContract,
            gates: std::collections::BTreeSet<Gate>,
            enforcement: BTreeMap<Control, EnforcementClaim>,
            #[serde(default)]
            effective_access: Option<AccessSnapshot>,
            #[serde(default)]
            limits: Option<ResourceLimits>,
            compiled_at: Timestamp,
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            binding: w.binding,
            role: w.role,
            profile: w.profile,
            host_id: w.host_id,
            host_revision: w.host_revision,
            task_id: w.task_id,
            root_id: w.root_id,
            stream_id: w.stream_id,
            task_revision: w.task_revision,
            task_contract: w.task_contract,
            gates: w.gates,
            enforcement: w.enforcement,
            effective_access: w.effective_access,
            limits: w.limits,
            compiled_at: w.compiled_at,
        };
        value
            .validate_at(value.compiled_at)
            .map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dispatch {
    id: DispatchId,
    contract_id: RuntimeContractId,
    contract: WorkforceRuntimeContract,
}

impl Dispatch {
    pub fn id(&self) -> &DispatchId {
        &self.id
    }
    pub fn contract(&self) -> &WorkforceRuntimeContract {
        &self.contract
    }

    /// Pure compilation from already authenticated, resolved records. The Host must
    /// resolve these from authoritative storage, not accept worker-supplied snapshots.
    pub fn compile(
        id: DispatchId,
        contract_id: RuntimeContractId,
        inputs: DispatchInputs<'_>,
    ) -> Result<Self, DomainError> {
        let DispatchInputs {
            task,
            role,
            binding,
            profile,
            host,
            limits,
            minimum_enforcement,
            now,
        } = inputs;
        if task.role_id() != &role.id
            || binding.role_id != role.id
            || task.project_id() != &role.project_id
            || binding.project_id != role.project_id
            || binding.access.project_id != role.project_id
            || binding.profile_id != profile.id
            || binding.profile_revision != profile.revision
        {
            return Err(DomainError::LineageMismatch);
        }
        if task.state() != &TaskState::Ready {
            return Err(DomainError::IllegalTransition);
        }
        if !profile.eligible_hosts.contains(&host.id)
            || !host.supported_runtimes.contains(&profile.runtime)
        {
            return Err(DomainError::IneligibleHost);
        }
        if !binding.access.grants.contains(&Permission::MutateStream)
            || !binding.access.roots.contains(task.root_id())
        {
            return Err(DomainError::PermissionDenied);
        }
        if binding.context.max_input_tokens == 0 || binding.context.reserved_output_tokens == 0 {
            return Err(DomainError::InvalidContextBudget);
        }
        if !limits.validate() {
            return Err(DomainError::ResourceLimit);
        }
        // Completion and mutation confinement are mandatory regardless of staffing.
        let mut required = binding.required_controls.clone();
        required.extend([
            Control::CompletionAuthority,
            Control::Filesystem,
            Control::Cancellation,
        ]);
        for (permission, control) in [
            (Permission::ExecuteProcess, Control::Process),
            (Permission::Network, Control::Network),
            (Permission::UseCredential, Control::Credentials),
        ] {
            if binding.access.grants.contains(&permission) {
                required.insert(control);
            }
        }
        let mut enforcement = BTreeMap::new();
        for control in required {
            let claim = host
                .controls
                .get(&control)
                .ok_or(DomainError::UnsupportedControl)?;
            if !matches!(
                claim.strength,
                EnforcementStrength::Native | EnforcementStrength::HostEnforced
            ) || claim.verified_at > now
                || claim.expires_at <= now
            {
                return Err(DomainError::UnsupportedControl);
            }
            // The binding's enforcement policy is binding at assignment: a
            // host claim weaker than the declared minimum refuses the
            // dispatch (re-staffing recompiles, so a fallback that cannot
            // meet the policy is rejected, never inherited).
            if let Some(minimum) = minimum_enforcement.get(&control) {
                if !strength_meets(minimum, &claim.strength) {
                    return Err(DomainError::UnsupportedControl);
                }
            }
            enforcement.insert(control, claim.clone());
        }
        // Least privilege per dispatch: the binding authorizes the Role's
        // whole scope, but THIS dispatch operates on exactly this Task's
        // Root — the narrow snapshot is what consumers (sandbox consent,
        // credential scope, context resolution) must enforce.
        let effective = {
            let mut access = binding.access.clone();
            access.roots = std::iter::once(task.root_id().clone()).collect();
            access
        };
        Ok(Self {
            id,
            contract_id,
            contract: WorkforceRuntimeContract {
                binding: binding.clone(),
                role: role.clone(),
                profile: profile.clone(),
                host_id: host.id.clone(),
                host_revision: host.revision,
                task_id: task.id().clone(),
                root_id: task.root_id().clone(),
                stream_id: task.stream_id().clone(),
                task_revision: task.revision(),
                task_contract: task.task_contract().clone(),
                gates: Gate::required(),
                enforcement,
                effective_access: Some(effective),
                limits: Some(limits.clone()),
                compiled_at: now,
            },
        })
    }
}

/// The strength ordering from the runtime SDK's minimum-enforcement check,
/// mirrored arm-for-arm against enforcement CLAIMS at assignment time.
/// ExternallyObserved and Emulated are incomparable in the SDK, so the
/// mirror keeps them distinct; at assignment the earlier gate already
/// rejects any claim weaker than HostEnforced, so for reachable claims the
/// distinguishing arms agree.
fn strength_meets(minimum: &EnforcementStrength, claim: &EnforcementStrength) -> bool {
    match minimum {
        EnforcementStrength::Native => claim == &EnforcementStrength::Native,
        EnforcementStrength::HostEnforced => matches!(
            claim,
            EnforcementStrength::Native | EnforcementStrength::HostEnforced
        ),
        EnforcementStrength::ExternallyObserved => matches!(
            claim,
            EnforcementStrength::Native
                | EnforcementStrength::HostEnforced
                | EnforcementStrength::ExternallyObserved
        ),
        EnforcementStrength::Emulated => matches!(
            claim,
            EnforcementStrength::Native
                | EnforcementStrength::HostEnforced
                | EnforcementStrength::Emulated
        ),
        EnforcementStrength::Unsupported => false,
    }
}

pub struct DispatchInputs<'a> {
    pub task: &'a Task,
    pub role: &'a Role,
    pub binding: &'a WorkforceBinding,
    pub profile: &'a RuntimeProfile,
    pub host: &'a Host,
    /// The staffing candidate's declared resource limits, recorded on the
    /// contract so the execution they authorize is bound by them.
    pub limits: &'a ResourceLimits,
    /// The binding's declared per-control enforcement floor. Controls the
    /// dispatch does not require are unconstrained by it.
    pub minimum_enforcement: &'a BTreeMap<Control, EnforcementStrength>,
    pub now: Timestamp,
}
