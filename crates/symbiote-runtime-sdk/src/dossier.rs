//! The Compatibility Dossier a Harness Pack publishes: the upstream versions it
//! was actually run against, and the evidence from each run.
//!
//! A tier, an adapter version and a `minimum_version` string in a manifest are
//! claims a reader has to take on trust. This module is the record behind them:
//! one [`ConformanceRun`] per upstream version and platform the pack was really
//! run against, each stamped from the [`RuntimeDescriptor`] the run was made
//! over, carrying the suite's own outcome and the suite's own two matrices. A
//! dossier is built *from* a [`ConformanceReport`](crate::conformance::ConformanceReport)
//! and refuses anything that report did not say, so a pack cannot publish a
//! capability its own run did not carry, a version it was not run against, or a
//! run that failed.
//!
//! The refusals are the contract, so they are named rather than implied:
//!
//! * [`DossierError::TheRunNamesAnotherRuntime`] — the report is not of the
//!   runtime it would be published under.
//! * [`DossierError::RulesFailed`] — the run itself failed a rule, so its
//!   evidence is not a certification.
//! * [`DossierError::NoDispatches`] — the run read no dispatch, so its matrices
//!   record no demand at all and prove nothing about any work.
//! * [`DossierError::MatrixAgainstTheDeclaration`] — a matrix row states
//!   something other than what that runtime's declaration says.
//! * [`DossierError::MatrixOmitsACarrier`] — a matrix omits a carrier or a
//!   control this crate places, so it hides one instead of declaring it absent.
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

use crate::conformance::{CapabilityRead, ConformanceReport, ControlRead, DispatchRead};
use crate::projection::{ContractSurface, carrier_delivery};
use crate::{IntegrationTier, RuntimeDescriptor, RuntimeOwner};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;
use symbiote_domain::{AgentRuntimeAdapterId, HarnessDriverId};

/// The version of the dossier shape itself. A dossier is a published document
/// with its own vocabulary, so it states which one it speaks rather than
/// leaving a reader to infer it from a field set.
pub const DOSSIER_SCHEMA_VERSION: u32 = 1;

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
    /// each stamped from the descriptor the run was made over — never supplied
    /// by the pack, which is the whole point: a version nobody ran against
    /// cannot appear here.
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
    /// Publishes the runs a pack produced. Every refusal here is a claim a
    /// reader would otherwise have to take on trust.
    pub fn publish(runs: Vec<ConformanceRun>) -> Result<Self, DossierError> {
        let first = runs.first().ok_or(DossierError::NoRuns)?;
        for run in &runs[1..] {
            if identity(run) != identity(first) {
                return Err(DossierError::MoreThanOnePack {
                    first: identity(first),
                    other: identity(run),
                });
            }
        }
        let mut seen = BTreeSet::new();
        for run in &runs {
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
            runs,
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
    TheRunNamesAnotherRuntime {
        declared: String,
        reported: String,
    },
    RulesFailed {
        rules: Vec<String>,
    },
    NoDispatches,
    MatrixAgainstTheDeclaration {
        carrier: String,
        stated: String,
        declared: String,
    },
    MatrixOmitsACarrier {
        missing: Vec<String>,
    },
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
            Self::TheRunNamesAnotherRuntime { declared, reported } => write!(
                f,
                "the run reports adapter {reported}, and a dossier cannot publish it under {declared}"
            ),
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
            Self::MatrixAgainstTheDeclaration {
                carrier,
                stated,
                declared,
            } => write!(
                f,
                "the matrix says {carrier} is {stated} where the declaration says {declared}"
            ),
            Self::MatrixOmitsACarrier { missing } => {
                write!(
                    f,
                    "the matrix omits {}: {}",
                    missing.len(),
                    missing.join(", ")
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
    /// together make it. Every field is read from one of the two: the versions,
    /// the platform and the driver from the descriptor, the outcomes and the
    /// matrices from the report, and both matrices are then read back against
    /// the declaration so a hand-edited row is refused rather than published.
    pub fn of(
        descriptor: &RuntimeDescriptor,
        report: &ConformanceReport,
    ) -> Result<Self, DossierError> {
        if report.adapter != descriptor.adapter_id || report.tier != descriptor.tier {
            return Err(DossierError::TheRunNamesAnotherRuntime {
                declared: descriptor.adapter_id.as_str().to_owned(),
                reported: report.adapter.as_str().to_owned(),
            });
        }
        if !report.passed() {
            return Err(DossierError::RulesFailed {
                rules: report.failures().into_iter().map(str::to_owned).collect(),
            });
        }
        if report.dispatches.is_empty() {
            return Err(DossierError::NoDispatches);
        }
        let declared_capabilities = expected_capabilities();
        let named: BTreeSet<String> = report
            .capabilities
            .iter()
            .map(|read| name(&read.capability))
            .collect();
        if declared_capabilities != named {
            return Err(DossierError::MatrixOmitsACarrier {
                missing: declared_capabilities.difference(&named).cloned().collect(),
            });
        }
        for read in &report.capabilities {
            let declared = carrier_delivery(descriptor.capabilities.get(&read.capability));
            if read.declared != declared {
                return Err(DossierError::MatrixAgainstTheDeclaration {
                    carrier: name(&read.capability),
                    stated: format!("{:?}", read.declared),
                    declared: format!("{declared:?}"),
                });
            }
        }
        let declared_controls = expected_controls();
        let named: BTreeSet<String> = report
            .controls
            .iter()
            .map(|read| name(&read.control))
            .collect();
        if declared_controls != named {
            return Err(DossierError::MatrixOmitsACarrier {
                missing: declared_controls.difference(&named).cloned().collect(),
            });
        }
        for read in &report.controls {
            let declared = descriptor.controls.get(&read.control);
            if read.declared != declared.map(|support| support.strength)
                || read.mechanism.as_deref() != declared.map(|support| support.mechanism.as_str())
            {
                return Err(DossierError::MatrixAgainstTheDeclaration {
                    carrier: name(&read.control),
                    stated: format!("{:?}", read.declared),
                    declared: format!("{:?}", declared.map(|support| support.strength)),
                });
            }
        }
        let driver = match &descriptor.owner {
            RuntimeOwner::External { driver } => Some(driver.clone()),
            RuntimeOwner::SymbioteNative {} => None,
        };
        Ok(Self {
            adapter: descriptor.adapter_id.clone(),
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
            driver,
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

/// Every capability this crate places on a surface, by the name a reader sees:
/// a matrix that omits one of them is hiding a carrier rather than declaring it
/// absent.
fn expected_capabilities() -> BTreeSet<String> {
    ContractSurface::ALL
        .iter()
        .flat_map(|surface| surface.capabilities())
        .map(name)
        .collect()
}

/// Every control this crate places on a surface, by the same names.
fn expected_controls() -> BTreeSet<String> {
    ContractSurface::ALL
        .iter()
        .flat_map(|surface| surface.controls())
        .map(name)
        .collect()
}

/// The name a refusal uses for a carrier or a claim: the same name the wire
/// carries, without the quotes the wire puts around it, so a reader can match a
/// failure to the row it names.
fn name<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .map(|wire| wire.trim_matches('"').to_owned())
        .unwrap_or_else(|_| String::from("unnamed"))
}
