//! Read-only prerequisite assessment. No result authorizes activation.
use crate::*;
use symbiote_host_inventory::{HostPulse, PulseRequirements};
use symbiote_runtime_sdk::{RuntimeDescriptor, qualify_profile_with_minimums};
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrerequisiteCheck {
    pub prerequisite: Prerequisite,
    pub result: CheckResult,
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
pub fn assess_readiness(
    configuration: &BindingConfiguration,
    team: &TeamConfiguration,
    pulse: Option<&HostPulse>,
    descriptor: Option<&RuntimeDescriptor>,
    now: Timestamp,
) -> ReadinessReport {
    let candidate = &configuration.primary;
    let mut checks = vec![PrerequisiteCheck {
        prerequisite: Prerequisite::CurrentTeam,
        result: if configuration.validate_team(team).is_ok() {
            CheckResult::Satisfied
        } else {
            CheckResult::Rejected
        },
    }];
    checks.push(PrerequisiteCheck {
        prerequisite: Prerequisite::HostCapacity,
        result: match pulse {
            None => CheckResult::MissingObservation,
            Some(pulse) => {
                let requirements = PulseRequirements {
                    host_id: pulse.host_id.clone(),
                    // Concurrency does not imply a CPU reservation.
                    minimum_cpu_millicores: None,
                    minimum_available_memory_bytes: Some(candidate.limits.max_memory_bytes),
                    capabilities: vec![],
                };
                if candidate.profile.eligible_hosts.contains(&pulse.host_id)
                    && pulse.qualify(&requirements, now).is_ok()
                {
                    CheckResult::Satisfied
                } else {
                    CheckResult::Rejected
                }
            }
        },
    });
    checks.push(PrerequisiteCheck {
        prerequisite: Prerequisite::RuntimeCapabilities,
        result: match descriptor {
            None => CheckResult::MissingObservation,
            Some(d) => {
                if pulse.is_some_and(|p| p.host_id == d.host_id)
                    && qualify_profile_with_minimums(
                        &candidate.profile,
                        d,
                        &configuration.policies.required_capabilities,
                        &configuration.policies.minimum_enforcement,
                        now,
                    )
                    .is_ok()
                {
                    CheckResult::Satisfied
                } else {
                    CheckResult::Rejected
                }
            }
        },
    });
    checks.push(PrerequisiteCheck {
        prerequisite: Prerequisite::RuntimeResources,
        result: match descriptor {
            None => CheckResult::MissingObservation,
            Some(d) => {
                if candidate.tools.is_subset(&d.tools)
                    && candidate.skills.is_subset(&d.skills)
                    && d.context_limits.as_ref().is_some_and(|limits| {
                        u64::from(limits.context_window_tokens)
                            >= u64::from(candidate.context.max_input_tokens)
                                + u64::from(candidate.context.reserved_output_tokens)
                            && limits.max_output_tokens >= candidate.context.reserved_output_tokens
                    })
                {
                    CheckResult::Satisfied
                } else {
                    CheckResult::Rejected
                }
            }
        },
    });
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
