//! The conformance suite every runtime adapter runs: one module, one set of
//! named rules, one report, whatever runtime sits behind the adapter.
//!
//! A pack's claim is a declaration — capabilities, enforcement strengths, a
//! support tier — and [`crate::projection`] is what reads that declaration onto
//! actual work. This module is where a declaration is *conformant*: every rule
//! has a name, a failure names the rule and the declared fact it failed on, and
//! the report carries the two matrices an operator or a pack publishes — one row
//! per capability, one row per control, each read off the adapter's own
//! projection rather than off the descriptor alone.
//!
//! What the suite decides is bounded on purpose, and stated rather than implied.
//! It reads a descriptor, an adapter's own projection of a dispatch, and the two
//! together — all pure, all in-process. It does not launch a session, observe a
//! process, or authenticate the evidence a descriptor references: launching needs
//! a [`LaunchPermit`](crate::LaunchPermit) that only an authorization mints, and
//! authenticating proof is the Host's job before any of this metadata is trusted.
//! A suite that pretended to prove either would be a second, weaker owner of
//! facts the Host and the sandbox already own.
//!
//! It does not decide eligibility either. A control a dispatch claims that the
//! descriptor never declares is a *row* in the enforcement matrix — declared
//! nowhere, demanded, not realized — and not a broken rule: whether that runtime
//! may run that dispatch is [`qualify_dispatch`](crate::qualify_dispatch)'s
//! refusal, and a suite that refused it here would be a second owner of the
//! decision. Freshness is the same: the window a descriptor's evidence spans is
//! read, the clock it is compared against is not.

use crate::projection::{CarrierDelivery, ContractProjection, ContractSurface, carrier_delivery};
use crate::*;
use std::collections::BTreeSet;
use symbiote_domain::{AgentRuntimeAdapterId, Control, Dispatch, DispatchId, EnforcementStrength};

/// One rule of the suite: the name a failure carries, and what an adapter must
/// satisfy. A rule may not be added, renamed or removed without the case that
/// holds it moving with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConformanceRule {
    /// The name a report and a failure carry.
    pub id: &'static str,
    /// What an adapter must satisfy, in one sentence.
    pub holds: &'static str,
}

/// The suite, in the order a report lists it.
pub const RULES: &[ConformanceRule] = &[
    ConformanceRule {
        id: "descriptor_identity_is_valid",
        holds: "the descriptor validates on its own terms: a known SDK version, bounded text, \
                and a runtime whose owner and transport are the ones that runtime can have",
    },
    ConformanceRule {
        id: "every_declared_capability_names_evidence_of_its_own",
        holds: "a capability declared supported carries evidence naming this exact adapter, \
                installation, profile, revision, model, host, versions and platform, with a \
                window that is not empty",
    },
    ConformanceRule {
        id: "every_declared_control_names_evidence_of_its_own",
        holds: "a declared control carries evidence of that same identity, a mechanism within \
                the module's bound, and a strength that is not the unsupported one",
    },
    ConformanceRule {
        id: "the_adapters_projection_names_the_runtime_its_own_descriptor_names",
        holds: "a projection names the dispatch and the runtime identity its own descriptor \
                names, and the canonical facts its dispatch was compiled under",
    },
    ConformanceRule {
        id: "every_projection_covers_every_surface_in_the_modules_order",
        holds: "a projection names every surface this module declares, once, in the module's own \
                order, so a surface cannot be dropped or reordered out of a report",
    },
    ConformanceRule {
        id: "every_demanded_capability_is_carried_exactly_as_it_is_declared",
        holds: "the carriers a projection reports are the ones its own descriptor's demand \
                implies: a demanded capability is carried exactly when it is declared supported, \
                and a carrier nothing demands is reported nowhere",
    },
    ConformanceRule {
        id: "every_demanded_control_is_read_at_the_strength_the_descriptor_declares",
        holds: "the claims a projection reports are the ones its own descriptor's declaration \
                implies: delivered at the declared strength when that realizes the claim, \
                degraded with both strengths when it does not, and missing when the descriptor \
                never lists it",
    },
    ConformanceRule {
        id: "reading_the_same_dispatch_twice_gives_the_same_projection",
        holds: "a projection is a pure read: the same dispatch yields the same projection, so a \
                caller cannot get a different answer by asking again",
    },
];

/// One rule's outcome. A failure is text a reader can act on and never carries
/// task content: it names the rule's own subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleOutcome {
    pub rule: ConformanceRule,
    /// What failed, or nothing when the rule held.
    pub failure: Option<String>,
}
impl RuleOutcome {
    /// Whether this rule held.
    pub fn held(&self) -> bool {
        self.failure.is_none()
    }
}

/// One capability the module places on a surface, read beside what the
/// adapter's own projections carry for it. This is the capability matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityRead {
    /// The carrier this row is about.
    pub capability: Capability,
    /// The descriptor's own declaration, in the projection's own vocabulary, so
    /// both readers agree on what four facts are four. A capability the
    /// descriptor never mentions is `Absent` rather than assumed.
    pub declared: CarrierDelivery,
    /// Whether any dispatch the suite ran demands this capability.
    pub demanded: bool,
    /// Whether every place that demands it reports it carried as declared. A
    /// capability no dispatch demands is read as carried in the sense its
    /// declaration states: nothing contradicts it.
    pub carried: bool,
}

/// One control the module places on a surface, with what the descriptor
/// declares and what the adapter's projections made of it. This is the
/// enforcement matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlRead {
    /// The claim this row is about.
    pub control: Control,
    /// The one surface this module places the control on.
    pub surface: ContractSurface,
    /// The strength the descriptor declares, or `None` when it does not list
    /// the control at all.
    pub declared: Option<EnforcementStrength>,
    /// The mechanism the descriptor names, or `None` when it lists no control.
    pub mechanism: Option<String>,
    /// Whether any dispatch the suite ran claims this control on its surface.
    pub demanded: bool,
    /// Whether every place that demands it reports it carried — delivered or
    /// degraded, since a degradation is realized. A control no dispatch demands
    /// is read as realized in the sense its declaration states: nothing
    /// contradicts it.
    pub realized: bool,
}

/// What one dispatch's projection published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchRead {
    /// The dispatch read.
    pub dispatch: DispatchId,
    /// How many surfaces the projection names.
    pub surfaces: usize,
    /// The surfaces the projection carries nothing for, in the module's order.
    pub withheld: Vec<ContractSurface>,
}

/// What the suite read, as data: a pack publishes this, an operator reads it,
/// and a failure names the rule and the declared fact it failed on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConformanceReport {
    /// The adapter the run was made against.
    pub adapter: AgentRuntimeAdapterId,
    /// The tier its descriptor claims. A tier implies nothing by itself, which
    /// is why the matrices beside it are the evidence.
    pub tier: IntegrationTier,
    /// One outcome per rule, in [`RULES`]' order.
    pub rules: Vec<RuleOutcome>,
    /// One row per capability this module places on a surface.
    pub capabilities: Vec<CapabilityRead>,
    /// One row per control this module places on a surface.
    pub controls: Vec<ControlRead>,
    /// One entry per dispatch the run was made over, in the caller's order.
    pub dispatches: Vec<DispatchRead>,
}
impl ConformanceReport {
    /// Whether every rule held. A report with one broken rule is not a pass.
    pub fn passed(&self) -> bool {
        self.rules.iter().all(RuleOutcome::held)
    }
    /// The rules that did not hold, by name.
    pub fn failures(&self) -> Vec<&'static str> {
        self.rules
            .iter()
            .filter(|outcome| !outcome.held())
            .map(|outcome| outcome.rule.id)
            .collect()
    }
    /// One rule's outcome, by name.
    pub fn rule(&self, id: &str) -> Option<&RuleOutcome> {
        self.rules.iter().find(|outcome| outcome.rule.id == id)
    }
    /// The capability's row, by name.
    pub fn capability(&self, capability: &Capability) -> Option<&CapabilityRead> {
        self.capabilities
            .iter()
            .find(|read| &read.capability == capability)
    }
    /// The control's row, by name.
    pub fn control(&self, control: &Control) -> Option<&ControlRead> {
        self.controls.iter().find(|read| &read.control == control)
    }
    /// The surfaces some dispatch's projection carried nothing for, in the
    /// module's order: the union, so a report says where this adapter is short
    /// across the dispatches it was run against.
    pub fn withheld(&self) -> Vec<ContractSurface> {
        ContractSurface::ALL
            .into_iter()
            .filter(|surface| {
                self.dispatches
                    .iter()
                    .any(|read| read.withheld.contains(surface))
            })
            .collect()
    }
}

/// Runs the whole suite over one adapter against the dispatches a caller
/// supplies.
///
/// The dispatches are the caller's: a conformance run is a run against *actual*
/// abstract work, and this module has no business inventing a task, a role or a
/// binding to project. Every rule is evaluated over every dispatch, and a
/// dispatch that no rule disagreed about still contributes its rows to the
/// matrices, so a report cannot pass because the dispatches happened to be easy.
pub fn run_conformance(
    adapter: &dyn AgentRuntimeAdapter,
    dispatches: &[&Dispatch],
) -> ConformanceReport {
    let descriptor = adapter.descriptor();
    let rule = |id: &str| {
        *RULES
            .iter()
            .find(|rule| rule.id == id)
            .expect("every rule named here is in the suite")
    };
    let mut outcomes = Vec::with_capacity(RULES.len());

    // The declaration, on its own terms.
    outcomes.push(match descriptor.validate() {
        Ok(()) => RuleOutcome {
            rule: rule("descriptor_identity_is_valid"),
            failure: None,
        },
        Err(error) => RuleOutcome {
            rule: rule("descriptor_identity_is_valid"),
            failure: Some(format!("the descriptor does not validate: {error:?}")),
        },
    });

    let mut capability_failure = None;
    for (capability, support) in &descriptor.capabilities {
        if let Support::Supported { evidence } = support {
            if let Some(problem) = evidence_problem(descriptor, evidence) {
                capability_failure =
                    Some(format!("{capability:?} is declared supported on {problem}"));
                break;
            }
        }
    }
    outcomes.push(outcome(
        rule("every_declared_capability_names_evidence_of_its_own"),
        capability_failure,
    ));

    let mut control_failure = None;
    for (control, support) in &descriptor.controls {
        if support.strength == EnforcementStrength::Unsupported {
            control_failure = Some(format!(
                "{control:?} is declared at the unsupported strength, which realizes nothing"
            ));
            break;
        }
        if support.mechanism.trim().is_empty() || support.mechanism.len() > MECHANISM_MAX_BYTES {
            control_failure = Some(format!(
                "{control:?} names a mechanism outside the module's bound"
            ));
            break;
        }
        if let Some(problem) = evidence_problem(descriptor, &support.evidence) {
            control_failure = Some(format!("{control:?} is declared on {problem}"));
            break;
        }
    }
    outcomes.push(outcome(
        rule("every_declared_control_names_evidence_of_its_own"),
        control_failure,
    ));

    // The projection the adapter publishes, read against the one its own
    // descriptor implies. The four rules below partition that comparison: a
    // disagreement in the header, in the surfaces, in a capability and in a
    // claim each fail by their own name, so a reader learns which fact moved.
    let mut header = None;
    let mut coverage = None;
    let mut carriers = None;
    let mut claims = None;
    let mut instability = None;
    let mut reads: Vec<Read> = Vec::with_capacity(dispatches.len());
    for dispatch in dispatches {
        let expected = ContractProjection::project(dispatch, descriptor);
        let actual = adapter.projection(dispatch);
        if adapter.projection(dispatch) != actual && instability.is_none() {
            instability = Some(format!(
                "projecting {} twice gave two different projections",
                dispatch.id().as_str()
            ));
        }
        if header.is_none() {
            header = header_problem(&actual, &expected);
        }
        if coverage.is_none() {
            coverage = coverage_problem(&actual);
        }
        if carriers.is_none() {
            carriers = carriers_problem(&actual, &expected);
        }
        if claims.is_none() {
            claims = claims_problem(&actual, &expected);
        }
        reads.push(Read { expected, actual });
    }
    outcomes.push(outcome(
        rule("the_adapters_projection_names_the_runtime_its_own_descriptor_names"),
        header,
    ));
    outcomes.push(outcome(
        rule("every_projection_covers_every_surface_in_the_modules_order"),
        coverage,
    ));
    outcomes.push(outcome(
        rule("every_demanded_capability_is_carried_exactly_as_it_is_declared"),
        carriers,
    ));
    outcomes.push(outcome(
        rule("every_demanded_control_is_read_at_the_strength_the_descriptor_declares"),
        claims,
    ));
    outcomes.push(outcome(
        rule("reading_the_same_dispatch_twice_gives_the_same_projection"),
        instability,
    ));

    // The matrices, read off the same projections: what each carrier's
    // declaration is, whether anything demanded it, and whether every place
    // that did reports it as its declaration states.
    let mut capabilities = Vec::new();
    let places: BTreeSet<Capability> = ContractSurface::ALL
        .iter()
        .flat_map(|surface| surface.capabilities())
        .cloned()
        .collect();
    for capability in places {
        let mut demanded = false;
        let mut carried = true;
        for read in &reads {
            for surface in &read.expected.surfaces {
                if !surface.carried.contains_key(&capability) {
                    continue;
                }
                demanded = true;
                let reported = read
                    .actual
                    .surface(surface.surface)
                    .and_then(|entry| entry.carried.get(&capability));
                if !matches!(reported, Some(CarrierDelivery::Declared {})) {
                    carried = false;
                }
            }
        }
        capabilities.push(CapabilityRead {
            declared: carrier_delivery(descriptor.capabilities.get(&capability)),
            capability,
            demanded,
            carried,
        });
    }

    let mut controls = Vec::new();
    for surface in ContractSurface::ALL {
        for control in surface.controls() {
            let declared = descriptor.controls.get(control);
            let mut demanded = false;
            let mut realized = true;
            for read in &reads {
                if !read
                    .expected
                    .surface(surface)
                    .is_some_and(|entry| entry.preventive.contains_key(control))
                {
                    continue;
                }
                demanded = true;
                let reported = read
                    .actual
                    .surface(surface)
                    .and_then(|entry| entry.preventive.get(control));
                if !reported.is_some_and(|delivery| delivery.realized().is_some()) {
                    realized = false;
                }
            }
            controls.push(ControlRead {
                control: control.clone(),
                surface,
                declared: declared.map(|support| support.strength),
                mechanism: declared.map(|support| support.mechanism.clone()),
                demanded,
                realized,
            });
        }
    }

    let dispatches = reads
        .iter()
        .map(|read| DispatchRead {
            dispatch: read.actual.dispatch.clone(),
            surfaces: read.actual.surfaces.len(),
            withheld: read.actual.withheld().into_iter().collect(),
        })
        .collect();

    ConformanceReport {
        adapter: descriptor.adapter_id.clone(),
        tier: descriptor.tier.clone(),
        rules: outcomes,
        capabilities,
        controls,
        dispatches,
    }
}

/// One dispatch's two readings: the projection the adapter published and the one
/// the module derives from its own descriptor.
struct Read {
    expected: ContractProjection,
    actual: ContractProjection,
}

fn outcome(rule: ConformanceRule, failure: Option<String>) -> RuleOutcome {
    RuleOutcome { rule, failure }
}

/// Whether one evidence reference names this descriptor's own identity and spans
/// a window that is not empty. The clock the window is compared against is
/// deliberately absent: freshness is qualification's fact, applied where the
/// descriptor is qualified rather than where a declaration is read.
fn evidence_problem(descriptor: &RuntimeDescriptor, evidence: &ProbeEvidence) -> Option<String> {
    if !descriptor.names_its_own_identity(evidence) {
        return Some(format!(
            "evidence `{}`, which does not name this descriptor's own identity",
            evidence.artifact.as_str()
        ));
    }
    if evidence.observed_at >= evidence.expires_at {
        return Some(format!(
            "evidence `{}`, whose observation window is empty",
            evidence.artifact.as_str()
        ));
    }
    None
}

/// The dispatch and runtime identity a projection names, and the canonical facts
/// it references. A projection read from a descriptor other than the one the
/// adapter hands out would be a second answer to the same question.
fn header_problem(actual: &ContractProjection, expected: &ContractProjection) -> Option<String> {
    let dispatch = expected.dispatch.as_str();
    if actual.dispatch != expected.dispatch {
        return Some(format!(
            "the projection of {dispatch} names dispatch {}",
            actual.dispatch.as_str()
        ));
    }
    if actual.adapter != expected.adapter
        || actual.installation != expected.installation
        || actual.profile != expected.profile
        || actual.profile_revision != expected.profile_revision
        || actual.model != expected.model
        || actual.host != expected.host
        || actual.runtime != expected.runtime
        || actual.adapter_version != expected.adapter_version
        || actual.upstream_version != expected.upstream_version
        || actual.platform != expected.platform
    {
        return Some(format!(
            "the projection of {dispatch} names a runtime other than the descriptor it was read \
             from"
        ));
    }
    if actual.references != expected.references {
        return Some(format!(
            "the projection of {dispatch} names canonical facts other than the ones its dispatch \
             was compiled under"
        ));
    }
    None
}

/// Whether a projection names every surface, once, in the module's own order.
fn coverage_problem(actual: &ContractProjection) -> Option<String> {
    let named: Vec<ContractSurface> = actual.surfaces.iter().map(|entry| entry.surface).collect();
    if named == ContractSurface::ALL {
        return None;
    }
    let missing: Vec<ContractSurface> = ContractSurface::ALL
        .into_iter()
        .filter(|surface| !named.contains(surface))
        .collect();
    if !missing.is_empty() {
        return Some(format!(
            "the projection names {} surfaces and omits {:?}",
            named.len(),
            missing
        ));
    }
    Some(format!(
        "the projection names every surface out of the module's order: {named:?}"
    ))
}

/// Whether the carriers a projection reports are the ones its own descriptor's
/// demand implies, read one capability at a time so a failure names the carrier
/// rather than the dispatch.
fn carriers_problem(actual: &ContractProjection, expected: &ContractProjection) -> Option<String> {
    for demanded in &expected.surfaces {
        let Some(reported) = actual.surface(demanded.surface) else {
            // The coverage rule owns a missing surface; this rule reads the
            // surfaces both projections name.
            continue;
        };
        for (capability, declared) in &demanded.carried {
            match reported.carried.get(capability) {
                Some(read) if read == declared => {}
                Some(read) => {
                    return Some(format!(
                        "{capability:?} is demanded on {:?} and reported {read:?} where its \
                         descriptor declares {declared:?}",
                        demanded.surface
                    ));
                }
                None => {
                    return Some(format!(
                        "{capability:?} is demanded on {:?} and the projection reports no \
                         carrier for it",
                        demanded.surface
                    ));
                }
            }
        }
        for capability in reported.carried.keys() {
            if !demanded.carried.contains_key(capability) {
                return Some(format!(
                    "{capability:?} is reported carried on {:?}, which demands no such carrier",
                    demanded.surface
                ));
            }
        }
    }
    None
}

/// Whether the claims a projection reports are the ones its own descriptor's
/// declaration implies, read one control at a time so a failure names the claim
/// rather than the dispatch.
fn claims_problem(actual: &ContractProjection, expected: &ContractProjection) -> Option<String> {
    for demanded in &expected.surfaces {
        let Some(reported) = actual.surface(demanded.surface) else {
            continue;
        };
        for (control, declared) in &demanded.preventive {
            match reported.preventive.get(control) {
                Some(read) if read == declared => {}
                Some(read) => {
                    return Some(format!(
                        "{control:?} is claimed on {:?} and reported {read:?} where its \
                         descriptor declares {declared:?}",
                        demanded.surface
                    ));
                }
                None => {
                    return Some(format!(
                        "{control:?} is claimed on {:?} and the projection reports no claim for \
                         it",
                        demanded.surface
                    ));
                }
            }
        }
        for control in reported.preventive.keys() {
            if !demanded.preventive.contains_key(control) {
                return Some(format!(
                    "{control:?} is reported on {:?}, which claims no such control",
                    demanded.surface
                ));
            }
        }
    }
    None
}
