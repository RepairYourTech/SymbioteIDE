//! Read-only prerequisite assessment. No result authorizes activation.
use crate::*;
use symbiote_host_inventory::{HostPulse, PulseError, PulseRequirements};
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
}
impl PrerequisiteCheck {
    fn new(prerequisite: Prerequisite, result: CheckResult) -> Self {
        Self {
            prerequisite,
            result,
            provider_refusal: None,
        }
    }
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
}
/// Assesses the primary candidate only. Fallback selection requires new explicit
/// consent and a separately compiled candidate assessment; none is selected here.
///
/// `provider` is the registry resolution the Host already performed for the
/// candidate's profile (`None` when the registry was not consulted). `descriptor`
/// is the Host's observation of the declared runtime for that profile (`None`
/// when the operator declared none). Both are inputs, never invented here: a
/// check whose observation is absent reports `MissingObservation` rather than a
/// satisfied prerequisite, and a check whose observation exists but does not
/// meet the requirement reports `Rejected` — the report never presents an
/// absence of evidence as a judgement against the Host.
pub fn assess_readiness(
    configuration: &BindingConfiguration,
    team: &TeamConfiguration,
    pulse: Option<&HostPulse>,
    descriptor: Option<&RuntimeDescriptor>,
    provider: Option<&ProviderResolution>,
    now: Timestamp,
) -> ReadinessReport {
    let candidate = &configuration.primary;
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
                    // Concurrency does not imply a CPU reservation.
                    minimum_cpu_millicores: None,
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
    ReadinessReport {
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
