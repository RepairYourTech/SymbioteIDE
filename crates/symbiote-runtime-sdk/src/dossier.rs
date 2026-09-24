//! The Compatibility Dossier a Harness Pack publishes: the upstream versions it
//! was actually run against, and the evidence from each run.
//!
//! A tier, an adapter version and a `minimum_version` string in a manifest are
//! claims a reader has to take on trust. This module is the record behind them:
//! one [`ConformanceRun`] per upstream version and platform the pack was really
//! run against, each carrying the suite's own outcome and the suite's own two
//! matrices.
//!
//! **A dossier is built from runs, not from records.**
//! [`CompatibilityDossier::publish`] takes the runtimes themselves and the
//! abstract work to run them over, evaluates the suite there, and reads the
//! versions, the platform, the tier and the driver from the descriptor it
//! evaluated. A caller has no parameter to hand a version, a matrix or an
//! outcome in through, so a pack cannot publish a capability its own run did not
//! carry, a version it was not run against, or a run that failed. A
//! `CompatibilityDossier` that arrives over a wire is a *reader's* input and not
//! a construction path: holding one against the descriptor the Host has is the
//! Host's check, and this crate does not pretend to have made it.
//!
//! The refusals are the contract, so they are named rather than implied:
//!
//! * [`DossierError::RulesFailed`] — the run itself failed a rule, so its
//!   evidence is not a certification. The refusal names the rules.
//! * [`DossierError::NoDispatches`] — the run read no dispatch, so its matrices
//!   record no demand at all and prove nothing about any work.
//! * [`DossierError::NoRuns`] — a dossier with no run certifies nothing.
//! * [`DossierError::MoreThanOnePack`] — the runs name two packs, and a dossier
//!   is one pack's record.
//! * [`DossierError::TheSameVersionTwice`] — the same upstream version on the
//!   same platform appears twice, so a reader cannot tell which of the two
//!   evidence rows is the claim.
//!
//! What a dossier does not do is decide anything: it is a record, and the Host
//! still qualifies a runtime against the declaration, and the Host still
//! authenticates proof. A dossier that claimed more would be a second owner.

use crate::conformance::{
    CapabilityRead, ConformanceReport, ControlRead, DispatchRead, run_conformance,
};
use crate::projection::ContractSurface;
use crate::{AgentRuntimeAdapter, IntegrationTier, RuntimeDescriptor, RuntimeOwner};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use symbiote_domain::{AgentRuntimeAdapterId, Dispatch, HarnessDriverId};

/// The version of the dossier shape itself. A dossier is a published document
/// with its own vocabulary, so it states which one it speaks rather than
/// leaving a reader to infer it from a field set.
pub const DOSSIER_SCHEMA_VERSION: u32 = 1;

/// One runtime a pack publishes evidence for, and the abstract work the suite is
/// run over. Both are borrowed: publication reads them and builds the record
/// itself, so a caller cannot hand in a run it wrote.
pub struct PackRun<'a> {
    /// The runtime itself. Its descriptor is what the suite reads, which is why
    /// the versions, the platform, the tier and the driver in the published run
    /// are this runtime's own.
    pub adapter: &'a dyn AgentRuntimeAdapter,
    /// The dispatches to run over: real abstract work, because a conformance
    /// run against work this crate invented would prove nothing.
    pub dispatches: Vec<&'a Dispatch>,
}

/// One rule's outcome as a dossier publishes it: the rule's name, and whether
/// it held. The prose a rule holds is the suite's own and does not travel on
/// the wire — a reader learns the name and looks it up in the SDK that ran it,
/// so a reworded rule cannot make a published dossier read as a different rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleRead {
    /// The suite's name for this rule, exactly as a report carried it.
    pub id: String,
    pub held: bool,
    /// What failed, when something did.
    pub failure: Option<String>,
}

/// One conformance run, as a pack publishes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConformanceRun {
    /// The adapter this run was of, and the harness driver it drives when it
    /// drives an external one.
    pub adapter: AgentRuntimeAdapterId,
    pub driver: Option<HarnessDriverId>,
    /// The upstream product version, the adapter's own version and the platform,
    /// each read from the descriptor the run was made over — never supplied by
    /// the pack, which is the whole point: a version nobody ran against cannot
    /// appear here.
    pub upstream_version: String,
    pub adapter_version: String,
    pub platform: String,
    /// The tier the runtime declared, which implies nothing by itself.
    pub tier: IntegrationTier,
    /// One outcome per rule the suite ran, in the suite's own order.
    pub rules: Vec<RuleRead>,
    /// The suite's capability matrix, as that run read it.
    pub capabilities: Vec<CapabilityRead>,
    /// The suite's enforcement matrix, as that run read it.
    pub controls: Vec<ControlRead>,
    /// What each dispatch in the run published.
    pub dispatches: Vec<DispatchRead>,
    /// The surfaces that run's projections carried nothing for, in this crate's
    /// own order.
    pub withheld: Vec<ContractSurface>,
}

/// A pack's published record of what it was run against.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityDossier {
    /// The dossier shape this document speaks.
    pub schema_version: u32,
    /// The pack this dossier is: its adapter, and the harness driver it drives
    /// when it drives an external one.
    pub adapter: AgentRuntimeAdapterId,
    pub driver: Option<HarnessDriverId>,
    /// One entry per upstream version and platform the pack was run against, in
    /// the order the pack published them.
    pub runs: Vec<ConformanceRun>,
}

impl CompatibilityDossier {
    /// Runs the suite over each runtime the pack names and publishes what it
    /// found. Every published field is read here: the versions, the platform,
    /// the tier and the driver from the descriptor, the outcomes and the two
    /// matrices from the report, and the withheld set from the same report. A
    /// caller that disagrees with any of it has nothing to hand in.
    pub fn publish(runs: Vec<PackRun<'_>>) -> Result<Self, DossierError> {
        let mut records = Vec::with_capacity(runs.len());
        for run in &runs {
            let report = run_conformance(run.adapter, &run.dispatches);
            records.push(ConformanceRun::of(run.adapter.descriptor(), &report)?);
        }
        let first = records.first().ok_or(DossierError::NoRuns)?;
        for run in &records[1..] {
            if identity(run) != identity(first) {
                return Err(DossierError::MoreThanOnePack {
                    first: identity(first),
                    other: identity(run),
                });
            }
        }
        let mut seen = BTreeSet::new();
        for run in &records {
            if !seen.insert((run.upstream_version.clone(), run.platform.clone())) {
                return Err(DossierError::TheSameVersionTwice {
                    upstream_version: run.upstream_version.clone(),
                    platform: run.platform.clone(),
                });
            }
        }
        Ok(Self {
            schema_version: DOSSIER_SCHEMA_VERSION,
            adapter: first.adapter.clone(),
            driver: first.driver.clone(),
            runs: records,
        })
    }

    /// The run for one upstream version on one platform, or `None` when this
    /// pack was never run against exactly that pair.
    pub fn run(&self, upstream_version: &str, platform: &str) -> Option<&ConformanceRun> {
        self.runs
            .iter()
            .find(|run| run.upstream_version == upstream_version && run.platform == platform)
    }

    /// The upstream versions this pack publishes evidence for, in its own order
    /// and without a hidden one: a version absent from this list is a version
    /// this dossier says nothing about.
    pub fn versions(&self) -> Vec<String> {
        self.runs
            .iter()
            .map(|run| run.upstream_version.clone())
            .collect()
    }
}

/// Why a dossier is refused. Every variant names the fact that made the
/// publication untrue, never the pack's intent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DossierError {
    RulesFailed {
        rules: Vec<String>,
    },
    NoDispatches,
    NoRuns,
    MoreThanOnePack {
        first: String,
        other: String,
    },
    TheSameVersionTwice {
        upstream_version: String,
        platform: String,
    },
}
impl fmt::Display for DossierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RulesFailed { rules } => {
                write!(
                    f,
                    "the run itself failed {}: {}",
                    rules.len(),
                    rules.join(", ")
                )
            }
            Self::NoDispatches => {
                write!(
                    f,
                    "the run read no dispatch, so its matrices record no demand"
                )
            }
            Self::NoRuns => write!(f, "a dossier with no run certifies nothing"),
            Self::MoreThanOnePack { first, other } => {
                write!(
                    f,
                    "{other} is not {first}, and a dossier is one pack's record"
                )
            }
            Self::TheSameVersionTwice {
                upstream_version,
                platform,
            } => write!(
                f,
                "upstream {upstream_version} on {platform} appears twice, and a reader cannot \
                 tell which row is the claim"
            ),
        }
    }
}
impl std::error::Error for DossierError {}

impl ConformanceRun {
    /// The run, as this runtime's declaration and the suite's report of it
    /// together make it. Private on purpose: this is the trusted construction
    /// boundary, and a record that a caller could build itself would be a way
    /// around it.
    fn of(
        descriptor: &RuntimeDescriptor,
        report: &ConformanceReport,
    ) -> Result<Self, DossierError> {
        if !report.passed() {
            return Err(DossierError::RulesFailed {
                rules: report.failures().into_iter().map(str::to_owned).collect(),
            });
        }
        if report.dispatches.is_empty() {
            return Err(DossierError::NoDispatches);
        }
        let driver = match &descriptor.owner {
            RuntimeOwner::External { driver } => Some(driver.clone()),
            RuntimeOwner::SymbioteNative {} => None,
        };
        Ok(Self {
            adapter: descriptor.adapter_id.clone(),
            driver,
            upstream_version: descriptor.upstream_version.clone(),
            adapter_version: descriptor.adapter_version.clone(),
            platform: descriptor.platform.clone(),
            tier: descriptor.tier.clone(),
            rules: report
                .rules
                .iter()
                .map(|outcome| RuleRead {
                    id: outcome.rule.id.to_owned(),
                    held: outcome.held(),
                    failure: outcome.failure.clone(),
                })
                .collect(),
            capabilities: report.capabilities.clone(),
            controls: report.controls.clone(),
            dispatches: report.dispatches.clone(),
            withheld: report.withheld(),
        })
    }
}

/// The pack a run belongs to: its adapter, and the driver it drives when it
/// drives an external one.
fn identity(run: &ConformanceRun) -> String {
    match &run.driver {
        Some(driver) => format!("{}/{}", run.adapter.as_str(), driver.as_str()),
        None => run.adapter.as_str().to_owned(),
    }
}
