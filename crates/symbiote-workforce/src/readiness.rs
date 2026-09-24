//! Read-only prerequisite assessment. No result authorizes activation.
use crate::*;
use symbiote_domain::{Permission, RuntimeKind, UnboundedLimit};
use symbiote_host_inventory::{HostPulse, PulseError, PulseRequirements};
use symbiote_runtime_sdk::projection::{BindingSurfaces, ContractSurface, binding_surfaces};
use symbiote_runtime_sdk::{QualificationError, RuntimeDescriptor, qualify_profile_with_minimums};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrerequisiteStatus {
    NotReady,
    ReadyForPreflight,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Prerequisite {
    CurrentTeam,
    HostCapacity,
    /// The provider registration the primary candidate's profile names, judged
    /// by the same registry validation the execution boundary uses.
    ProviderRegistration,
    RuntimeCapabilities,
    RuntimeResources,
    /// Whether this Host can honour the limits the candidate declares at all:
    /// the same decision activation makes before it runs anything
    /// (`ResourceLimits::process_bound`). A declared limit the Host has no
    /// kernel bound for is not a capacity question — the Host may well have
    /// the capacity and still refuse the run — so it is reported here, with the
    /// reason the run will refuse on, instead of being left to contradict a
    /// satisfied capacity check.
    EnforceableLimits,
    /// Whether the observed runtime carries, on the surfaces the projection
    /// places them, the facts this binding demands: the enforcement minimums its
    /// policy declares, the capabilities that policy requires, and the tools and
    /// skills the binding names — which the dispatch boundary requires the same
    /// way, so a report that omitted them would contradict the decision
    /// activation makes. The refusal names the withheld surfaces, so an operator
    /// reads *where* the runtime carries nothing rather than only that
    /// something is missing.
    RuntimeSurfaces,
    /// Whether the candidate's own lane can execute at all given the access its
    /// snapshot grants. The external lane's dispatch IS a Host-launched
    /// sandboxed process, and it can never acquire the process grant later
    /// (capability elevation refuses an `EXTERNAL_HARNESS` dispatch), so a
    /// candidate for that lane which does not grant `ExecuteProcess` is
    /// `rejected` here instead of being reported ready and then refused by the
    /// sandbox at the transport build.
    ExecutionAccess,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckResult {
    Satisfied,
    MissingObservation,
    Rejected,
}
/// The one classification every prerequisite shares: the Host holds no current
/// observation of the fact the prerequisite names (`missing_observation`), or it
/// observed the fact and the fact does not meet the requirement (`rejected`).
/// Neither is ever reported as the other, and only an observed fact that meets
/// the requirement is `satisfied`.
///
/// Pulse rejections that mean the Host has not observed the fact: a disabled or
/// non-operating-system probe, an expired sample and an unknown required value
/// are absences of evidence. Every other rejection is evidence against.
fn capacity_verdict(error: PulseError) -> CheckResult {
    match error {
        PulseError::Unknown | PulseError::Disabled | PulseError::Stale => {
            CheckResult::MissingObservation
        }
        _ => CheckResult::Rejected,
    }
}
/// Qualification rejections that mean the Host holds no usable observation of
/// the runtime: evidence that expired, evidence this SDK cannot read, and a
/// capability the observation itself reports as unknown. Every other rejection
/// is an observed fact that does not qualify.
fn runtime_verdict(error: QualificationError) -> CheckResult {
    match error {
        QualificationError::StaleEvidence
        | QualificationError::UnsupportedVersion
        | QualificationError::UnknownCapability(_) => CheckResult::MissingObservation,
        _ => CheckResult::Rejected,
    }
}
/// Whether the candidate's lane can execute at all given the access its
/// snapshot grants. Candidate validation already guarantees the Root grant and
/// a non-empty authorized root set, so the lane decides the rest: the external
/// lane always launches the Host-owned harness process and therefore needs
/// `ExecuteProcess`, while the native lane may run without one — a run whose
/// contract starts no sandboxed process starts no Host process either — so
/// demanding it there would reject a lane that would work.
fn execution_access_ok(candidate: &StaffingCandidate) -> bool {
    candidate.profile.runtime != RuntimeKind::ExternalHarness
        || candidate
            .access
            .grants
            .contains(&Permission::ExecuteProcess)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteCheck {
    pub prerequisite: Prerequisite,
    pub result: CheckResult,
    /// Present exactly on the provider prerequisite, and only when the registry
    /// refused: the refusal's own name, so the report says why a dispatch would
    /// not run rather than merely that it would not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_refusal: Option<ProviderRefusal>,
    /// Present exactly on the enforceable-limits prerequisite, and only when a
    /// declared limit cannot be bound: the declared limit in the one vocabulary
    /// activation refuses on, so the report names the reason the run will give
    /// rather than leaving the operator to discover it by starting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_refusal: Option<UnboundedLimit>,
    /// Present exactly on the runtime-surfaces prerequisite, and only when the
    /// observed runtime carries nothing for a surface the binding demands: the
    /// surfaces themselves, in their declared order, so the refusal names the
    /// places rather than a count.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub withheld_surfaces: Vec<ContractSurface>,
}
impl PrerequisiteCheck {
    fn new(prerequisite: Prerequisite, result: CheckResult) -> Self {
        Self {
            prerequisite,
            result,
            provider_refusal: None,
            limit_refusal: None,
            withheld_surfaces: Vec::new(),
        }
    }
}
/// The declared runtimes a Host observed for the candidates a binding names, in
/// the binding's own order: the primary first, then the fallbacks as the binding
/// lists them. Pairing the two here is what keeps a report from reading a
/// fallback's runtime onto the primary — [`CandidateObservations::for_configuration`]
/// refuses a list that does not hold exactly one entry per candidate rather than
/// reading the mismatch as an absence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateObservations<'a> {
    observations: Vec<Option<&'a RuntimeDescriptor>>,
}
impl<'a> CandidateObservations<'a> {
    /// The primary's observation and the fallbacks', in the binding's own order.
    pub fn new(
        primary: Option<&'a RuntimeDescriptor>,
        fallbacks: impl IntoIterator<Item = Option<&'a RuntimeDescriptor>>,
    ) -> Self {
        let mut observations = vec![primary];
        observations.extend(fallbacks);
        Self { observations }
    }
    /// Pairs what the Host observed with the candidates the binding names.
    /// `None` means the list does not hold one entry per candidate — a caller
    /// that read one binding's declarations onto another — so the report is not
    /// built rather than built from the wrong descriptor.
    pub fn for_configuration(
        configuration: &BindingConfiguration,
        observed: &'a [Option<RuntimeDescriptor>],
    ) -> Option<Self> {
        if observed.len() != 1 + configuration.fallbacks.len() {
            return None;
        }
        Some(Self::new(
            observed[0].as_ref(),
            observed[1..].iter().map(Option::as_ref),
        ))
    }
    /// The candidate at `index` in the binding's own order: index zero is the
    /// primary, the fallbacks follow.
    pub fn candidate(&self, index: usize) -> Option<&'a RuntimeDescriptor> {
        self.observations.get(index).copied().flatten()
    }
    /// The candidate the prerequisite checks assess.
    pub fn primary(&self) -> Option<&'a RuntimeDescriptor> {
        self.candidate(0)
    }
}
/// What this Host holds for one candidate the binding names. The three cases are
/// the three facts, never collapsed: an operator who declared nothing, an
/// observation this Host cannot attribute to itself, and this Host's own
/// observation of the runtime it would activate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CandidateObservation {
    /// The operator declared no runtime for this candidate's profile.
    NotDeclared,
    /// A runtime was observed for this candidate's profile, but not by this
    /// Host's own pulse. An observation this Host cannot attribute to itself is
    /// not a statement about the runtime it would activate.
    Unattributed,
    /// This Host's observation of the runtime it declared for this profile, read
    /// onto the binding's demand: the carriage names, per surface, what the
    /// declaration carries for every control the binding declares a minimum for
    /// and every carrier its policy or named resources require.
    Observed { surfaces: Box<BindingSurfaces> },
}
/// One candidate the binding may be staffed with, and what this Host holds for
/// it. The primary is first and the fallbacks follow in the binding's own order.
/// The report publishes this and selects nothing: whether a fallback may be used
/// is the binding's own consent policy, and the choice is activation's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandidateSurfaces {
    pub profile: RuntimeProfileId,
    pub profile_revision: Revision,
    pub model: ModelId,
    pub runtime: RuntimeKind,
    pub adapter: AgentRuntimeAdapterId,
    pub observation: CandidateObservation,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivationGate {
    EnvironmentResolution,
    AccessAndResourceConsent,
    ProviderAndModelAvailability,
    ResourceReservation,
    BudgetEnforcement,
    DispatchSnapshot,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadinessReport {
    pub status: PrerequisiteStatus,
    pub profile_id: RuntimeProfileId,
    pub checks: Vec<PrerequisiteCheck>,
    pub activation_pending: Vec<ActivationGate>,
    /// What this Host holds for each candidate the binding names, the primary
    /// first and the fallbacks in the binding's own order — one entry per
    /// candidate, so a report cannot silently omit one. The routing and
    /// eligibility read this instead of inferring carriage from a runtime's
    /// kind: for each candidate, the twelve surfaces, the demand the binding
    /// itself makes, and what the declaration carries for every one of them.
    pub candidates: Vec<CandidateSurfaces>,
}
/// Assesses the primary candidate: the prerequisite checks and the status are
/// that candidate's, and so is `profile_id`. Every candidate the binding names
/// is read onto its own observation in `candidates`, which is a statement about
/// carriage and nothing more — publishing what a fallback would carry is not
/// selecting it. Fallback selection requires new explicit consent and a
/// separately compiled candidate assessment; none is selected here, and no
/// consent is recorded in this model yet, so none could be read here.
///
/// `provider` is the registry resolution the Host already performed for the
/// candidate's profile (`None` when the registry was not consulted).
/// `observations` is the Host's own read of the declared runtimes, paired with
/// the candidates the binding names (`None` inside it when the operator declared
/// none for that profile). Both are inputs, never invented here: a check whose
/// observation is absent reports `MissingObservation` rather than a satisfied
/// prerequisite, and a check whose observation exists but does not meet the
/// requirement reports `Rejected` — the report never presents an absence of
/// evidence as a judgement against the Host.
pub fn assess_readiness(
    configuration: &BindingConfiguration,
    team: &TeamConfiguration,
    pulse: Option<&HostPulse>,
    observations: &CandidateObservations<'_>,
    provider: Option<&ProviderResolution>,
    now: Timestamp,
) -> ReadinessReport {
    let candidate = &configuration.primary;
    // Every candidate the binding names, read onto the runtime this Host
    // observed for that candidate's own profile. This is the one read: the
    // primary's entry is what the surface check below judges, so the report
    // cannot state a carriage twice.
    let candidates = std::iter::once(&configuration.primary)
        .chain(&configuration.fallbacks)
        .enumerate()
        .map(|(index, candidate)| CandidateSurfaces {
            profile: candidate.profile.id.clone(),
            profile_revision: candidate.profile.revision,
            model: candidate.profile.model.clone(),
            runtime: candidate.profile.runtime,
            adapter: candidate.profile.adapter.clone(),
            observation: match observations.candidate(index) {
                None => CandidateObservation::NotDeclared,
                // A descriptor observed by another Host — or observed while this
                // Host holds no pulse at all — is not this Host's observation of
                // the runtime it would activate.
                Some(descriptor)
                    if !pulse.is_some_and(|pulse| pulse.host_id == descriptor.host_id) =>
                {
                    CandidateObservation::Unattributed
                }
                Some(descriptor) => CandidateObservation::Observed {
                    surfaces: Box::new(binding_surfaces(
                        &configuration.binding,
                        &configuration.policies.minimum_enforcement,
                        &configuration.policies.required_capabilities,
                        descriptor,
                    )),
                },
            },
        })
        .collect::<Vec<_>>();
    let assessed = candidates.first().map(|entry| &entry.observation);
    // The runtime the prerequisite checks judge: the primary's own observation.
    let descriptor = observations.primary();
    let mut checks = vec![PrerequisiteCheck::new(
        Prerequisite::CurrentTeam,
        if configuration.validate_team(team).is_ok() {
            CheckResult::Satisfied
        } else {
            CheckResult::Rejected
        },
    )];
    checks.push(PrerequisiteCheck::new(
        Prerequisite::HostCapacity,
        match pulse {
            None => CheckResult::MissingObservation,
            // Eligibility is the Host's own fact, observed with the pulse: an
            // ineligible Host is a judgement, not an absence.
            Some(pulse) if !candidate.profile.eligible_hosts.contains(&pulse.host_id) => {
                CheckResult::Rejected
            }
            Some(pulse) => {
                let requirements = PulseRequirements {
                    host_id: pulse.host_id.clone(),
                    // Only a declared CPU demand is judged: concurrency does
                    // not imply a CPU reservation, so a candidate that
                    // declares none is not asking the Host for capacity it
                    // never expressed.
                    minimum_cpu_millicores: candidate.limits.max_cpu_millicores,
                    minimum_available_memory_bytes: Some(candidate.limits.max_memory_bytes),
                    capabilities: vec![],
                };
                match pulse.qualify(&requirements, now) {
                    Ok(()) => CheckResult::Satisfied,
                    Err(error) => capacity_verdict(error),
                }
            }
        },
    ));
    checks.push(PrerequisiteCheck {
        prerequisite: Prerequisite::ProviderRegistration,
        result: match provider {
            None => CheckResult::MissingObservation,
            Some(resolution) if resolution.refusal.is_none() => CheckResult::Satisfied,
            Some(_) => CheckResult::Rejected,
        },
        provider_refusal: provider.and_then(|resolution| resolution.refusal),
        limit_refusal: None,
        withheld_surfaces: Vec::new(),
    });
    checks.push(PrerequisiteCheck::new(
        Prerequisite::RuntimeCapabilities,
        match descriptor {
            None => CheckResult::MissingObservation,
            // A descriptor observed on another Host is not an observation of
            // this Host's runtime.
            Some(d) if !pulse.is_some_and(|p| p.host_id == d.host_id) => CheckResult::Rejected,
            Some(d) => match qualify_profile_with_minimums(
                &candidate.profile,
                d,
                &configuration.policies.required_capabilities,
                &configuration.policies.minimum_enforcement,
                now,
            ) {
                Ok(()) => CheckResult::Satisfied,
                Err(error) => runtime_verdict(error),
            },
        },
    ));
    checks.push(PrerequisiteCheck::new(
        Prerequisite::RuntimeResources,
        match descriptor {
            None => CheckResult::MissingObservation,
            Some(d) => {
                if !candidate.tools.is_subset(&d.tools) || !candidate.skills.is_subset(&d.skills) {
                    // The observation says the runtime does not carry a tool
                    // or skill the candidate declares.
                    CheckResult::Rejected
                } else {
                    match &d.context_limits {
                        // No observed context bounds: an absence, not an
                        // insufficient runtime.
                        None => CheckResult::MissingObservation,
                        Some(limits)
                            if u64::from(limits.context_window_tokens)
                                >= u64::from(candidate.context.max_input_tokens)
                                    + u64::from(candidate.context.reserved_output_tokens)
                                && limits.max_output_tokens
                                    >= candidate.context.reserved_output_tokens =>
                        {
                            CheckResult::Satisfied
                        }
                        Some(_) => CheckResult::Rejected,
                    }
                }
            }
        },
    ));
    // The decision activation makes, asked before the run: a declared limit
    // this Host cannot bind makes the candidate unusable, whatever capacity the
    // Host has, and is reported with the same reason the start refuses on. The
    // two boundaries therefore cannot disagree — a report that is ready for
    // preflight is a report whose every declared limit the Host can bind.
    let enforceable = match candidate.limits.process_bound() {
        Ok(_) => PrerequisiteCheck::new(Prerequisite::EnforceableLimits, CheckResult::Satisfied),
        Err(limit) => PrerequisiteCheck {
            prerequisite: Prerequisite::EnforceableLimits,
            result: CheckResult::Rejected,
            provider_refusal: None,
            limit_refusal: Some(limit),
            withheld_surfaces: Vec::new(),
        },
    };
    checks.push(enforceable);
    // What the candidate's own lane needs in order to execute at all, judged by
    // the same grants the sandbox checks before it launches a process. The
    // external lane always launches one and can never be granted it later, so a
    // candidate for it whose access does not carry the grant is rejected here
    // rather than reported ready and refused at the transport build.
    checks.push(PrerequisiteCheck::new(
        Prerequisite::ExecutionAccess,
        if execution_access_ok(candidate) {
            CheckResult::Satisfied
        } else {
            CheckResult::Rejected
        },
    ));
    // What the runtime declares for the surfaces this binding demands. The
    // observation is the same one the capability check reads, and the demand is
    // the binding's own, so this reports *where* the runtime carries the facts
    // rather than adding a second verdict on whether it may run: a runtime the
    // capability check refuses is refused there, and a runtime that declares
    // nothing for a demanded surface is named here with the surfaces it
    // withholds.
    checks.push(PrerequisiteCheck {
        prerequisite: Prerequisite::RuntimeSurfaces,
        result: match assessed {
            Some(CandidateObservation::Observed { surfaces }) if surfaces.is_complete() => {
                CheckResult::Satisfied
            }
            Some(CandidateObservation::Observed { .. }) => CheckResult::Rejected,
            // An observation this Host cannot attribute to itself is a
            // judgement against the runtime it would activate; nothing declared
            // at all is an absence.
            Some(CandidateObservation::Unattributed) => CheckResult::Rejected,
            Some(CandidateObservation::NotDeclared) | None => CheckResult::MissingObservation,
        },
        provider_refusal: None,
        limit_refusal: None,
        withheld_surfaces: match assessed {
            Some(CandidateObservation::Observed { surfaces }) => {
                surfaces.withheld().into_iter().collect()
            }
            _ => Vec::new(),
        },
    });
    ReadinessReport {
        candidates,
        status: if checks.iter().all(|c| c.result == CheckResult::Satisfied) {
            PrerequisiteStatus::ReadyForPreflight
        } else {
            PrerequisiteStatus::NotReady
        },
        profile_id: candidate.profile.id.clone(),
        checks,
        activation_pending: vec![
            ActivationGate::EnvironmentResolution,
            ActivationGate::AccessAndResourceConsent,
            ActivationGate::ProviderAndModelAvailability,
            ActivationGate::ResourceReservation,
            ActivationGate::BudgetEnforcement,
            ActivationGate::DispatchSnapshot,
        ],
    }
}
