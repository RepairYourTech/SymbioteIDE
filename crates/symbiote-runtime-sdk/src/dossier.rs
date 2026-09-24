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
//! a construction path: holding one against the runtime the Host has is the
//! Host's check, [`CompatibilityDossier::hold`] is the comparison it makes, and
//! this crate does not pretend to have made the decision.
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
//! **A published record is a claim, and holding it is the reader's check.**
//! Nothing about a parsed document can be taken on trust: it is a file somebody
//! wrote, and the facts it states about outcomes are exactly the facts a reader
//! cannot observe. [`CompatibilityDossier::hold`] therefore takes the runtime the
//! reader actually has — a [`DeclaredRuntime`], the form a Host holds its
//! operator's assertions in — and serves the record only when it is that pack's,
//! speaks the [`DOSSIER_SCHEMA_VERSION`] this crate speaks, shows an outcome
//! that held for every rule [`RULES`](crate::conformance::RULES) names, carries
//! the two matrices this crate's suite publishes row for row, has a run over
//! real work, has exactly one run per version and platform, and has one for the
//! upstream version and platform that runtime declares, made with that runtime's
//! own adapter version. Its refusals name what the document said, never what a
//! reader wished it said: `UnknownShape`, `AnotherPack`, `TheRunNamesAnotherPack`,
//! `UnknownRules`, `TheSameRunTwice`, `ARuleDidNotHold`,
//! `TheMatricesAreNotTheSuites`, `TheRunReadNoDispatch`, `NoRun` and
//! `AnotherAdapterVersion`.
//!
//! What holding a record is not, is stated rather than implied. It is not
//! qualification: a held record says this pack passed this suite at this version
//! on this platform, and the Host still decides whether that runtime may run a
//! dispatch. It is not authentication: the pack's own evidence artifacts are
//! still the Host's to verify, and a record that names an artifact proves
//! nothing until it does. And a tier is a label, so a held record's tier is
//! served as the label it is and is not compared as evidence.
//!
//! The limit of the check is stated here rather than left for a reader to
//! assume: what is compared is the record's *shape* — which rules, which
//! capability rows, which surface each control row is on, which pack the
//! document is — and the outcomes it reports. What a row *says* about a run
//! cannot be recomputed here, because a record carries each dispatch's summary
//! rather than the work the suite read, so those values are served as the pack's
//! claim. A reader who needs them observed rather than claimed runs the suite
//! itself, over real work, on the runtime it has.
//!
//! What a dossier does not do is decide anything: it is a record, and the Host
//! still qualifies a runtime against the declaration, and the Host still
//! authenticates proof. A dossier that claimed more would be a second owner.

use crate::conformance::{
    CapabilityRead, ConformanceReport, ControlRead, DispatchRead, RULES, placed_capabilities,
    placed_controls, run_conformance,
};
use crate::projection::ContractSurface;
use crate::{
    AgentRuntimeAdapter, DeclaredRuntime, IntegrationTier, RuntimeDescriptor, RuntimeOwner,
};
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

    /// Whether this published record is the pack these facts name: its own
    /// adapter, and the driver it drives when it drives an external one. Read
    /// the same way twice on purpose — by a Host choosing which installed file
    /// to serve and by [`Self::hold`] refusing a record that is another pack's
    /// — so the choice and the check cannot disagree about which file is whose.
    pub fn publishes(
        &self,
        adapter: &AgentRuntimeAdapterId,
        driver: Option<&HarnessDriverId>,
    ) -> bool {
        &self.adapter == adapter && self.driver.as_ref() == driver
    }

    /// Holds this record against the runtime a reader actually has.
    ///
    /// The reader's side of the document, and the only side that reads a parsed
    /// one: the pack identity, the shape, the rule outcomes, both matrices and
    /// the run for the runtime's own declared version and platform are all facts
    /// this file asserts about itself, so all of them are compared rather than
    /// believed. A refusal names the fact; a pass is a plain reading, never a
    /// promotion — see [`HoldError`] for what each refusal is, and the module
    /// header for what holding a record is not.
    pub fn hold<'a>(
        &'a self,
        declared: &'a DeclaredRuntime,
    ) -> Result<DossierHolding<'a>, HoldError> {
        if self.schema_version != DOSSIER_SCHEMA_VERSION {
            return Err(HoldError::UnknownShape {
                stated: self.schema_version,
            });
        }
        let driver = match &declared.owner {
            RuntimeOwner::External { driver } => Some(driver),
            RuntimeOwner::SymbioteNative {} => None,
        };
        if !self.publishes(&declared.adapter_id, driver) {
            return Err(HoldError::AnotherPack {
                stated: pack(&self.adapter, self.driver.as_ref()),
                held: pack(&declared.adapter_id, driver),
            });
        }
        let published = pack(&self.adapter, self.driver.as_ref());
        let capabilities = placed_capabilities();
        let controls = placed_controls();
        // One row per version and platform, exactly as publication guarantees:
        // a record with two runs for the same pair is one claim twice, and the
        // pair a reader looks a run up with would no longer name one run.
        let mut versions: BTreeSet<(&str, &str)> = BTreeSet::new();
        for run in &self.runs {
            if !versions.insert((&run.upstream_version, &run.platform)) {
                return Err(HoldError::TheSameRunTwice {
                    upstream_version: run.upstream_version.clone(),
                    platform: run.platform.clone(),
                });
            }
            if pack(&run.adapter, run.driver.as_ref()) != published {
                return Err(HoldError::TheRunNamesAnotherPack {
                    stated: pack(&run.adapter, run.driver.as_ref()),
                    pack: published,
                });
            }
            // A rule name this suite does not publish is not evidence about
            // anything: it is a row a document added, and serving the record
            // whole would serve it as one of the suite's outcomes.
            let unknown: Vec<String> = run
                .rules
                .iter()
                .filter(|row| !RULES.iter().any(|rule| rule.id == row.id))
                .map(|row| row.id.clone())
                .collect();
            if !unknown.is_empty() {
                return Err(HoldError::UnknownRules { rules: unknown });
            }
            // A rule this suite runs and the record does not show as exactly one
            // held row is one rule that did not hold on the evidence published: no
            // row, a row under another name, a row reporting a failure, a row
            // contradicting itself, and a second row beside a held one are five
            // ways of saying the same thing, and none of them is a rule the suite
            // passed. Two rows cannot be averaged into one outcome.
            let unheld: Vec<String> = RULES
                .iter()
                .filter(|rule| {
                    let mut rows = run.rules.iter().filter(|row| row.id == rule.id);
                    !matches!(
                        (rows.next(), rows.next()),
                        (Some(row), None) if row.held && row.failure.is_none()
                    )
                })
                .map(|rule| rule.id.to_owned())
                .collect();
            if !unheld.is_empty() {
                return Err(HoldError::ARuleDidNotHold { rules: unheld });
            }
            if !run
                .capabilities
                .iter()
                .map(|row| &row.capability)
                .eq(capabilities.iter())
            {
                return Err(HoldError::TheMatricesAreNotTheSuites {
                    what: "capabilities",
                });
            }
            if !run
                .controls
                .iter()
                .map(|row| (row.surface, row.control.clone()))
                .eq(controls.iter().cloned())
            {
                return Err(HoldError::TheMatricesAreNotTheSuites { what: "controls" });
            }
            if run.dispatches.is_empty() {
                return Err(HoldError::TheRunReadNoDispatch);
            }
        }
        let run = self
            .run(&declared.upstream_version, &declared.platform)
            .ok_or_else(|| HoldError::NoRun {
                upstream_version: declared.upstream_version.clone(),
                platform: declared.platform.clone(),
            })?;
        if run.adapter_version != declared.adapter_version {
            return Err(HoldError::AnotherAdapterVersion {
                stated: run.adapter_version.clone(),
                held: declared.adapter_version.clone(),
            });
        }
        Ok(DossierHolding { run, declared })
    }
}

/// What a reader learned by holding a published record against the runtime it
/// actually has: the run inside that record which certifies this runtime, and
/// the declaration it was held against.
///
/// Both are borrowed from what the reader already holds. A holding names the
/// run and the runtime it was made from rather than copying either, so it cannot
/// go on to describe a different run or a different runtime than the one the
/// check actually compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DossierHolding<'a> {
    pub run: &'a ConformanceRun,
    pub declared: &'a DeclaredRuntime,
}

/// Why a reader refuses to serve a published record. Every variant names a fact
/// the document itself states, never the reader's intent and never the pack's
/// story about why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoldError {
    /// The document speaks a dossier shape this crate does not, so its fields
    /// cannot be read as the ones this crate publishes.
    UnknownShape { stated: u32 },
    /// The record is another pack's than the runtime the reader holds.
    AnotherPack { stated: String, held: String },
    /// A run inside the record names a pack the record itself does not, so the
    /// document mixes packs and no run in it can be read as this pack's.
    TheRunNamesAnotherPack { stated: String, pack: String },
    /// A rule row the record carries under a name this suite does not publish:
    /// the outcome it claims is about a rule nobody here runs.
    UnknownRules { rules: Vec<String> },
    /// The same upstream version on the same platform appears in two runs, so
    /// the pair a reader looks a run up with no longer names one run.
    TheSameRunTwice {
        upstream_version: String,
        platform: String,
    },
    /// The record does not show these rules as held: a rule with no row, a row
    /// under another name, a row reporting a failure, and a row claiming an
    /// outcome beside a failure are the same absence of evidence.
    ARuleDidNotHold { rules: Vec<String> },
    /// One of the two matrices is not the one this suite publishes: a row is
    /// missing, extra, or on another surface, so what it says about demand is
    /// not what the suite read.
    TheMatricesAreNotTheSuites { what: &'static str },
    /// A run read no dispatch, so its matrices record no demand and it proves
    /// nothing about any work.
    TheRunReadNoDispatch,
    /// The pack publishes no run for the upstream version and platform this
    /// runtime declares, so this record says nothing about it.
    NoRun {
        upstream_version: String,
        platform: String,
    },
    /// The run for that version and platform was made with another build of the
    /// adapter, so it is not evidence about this one.
    AnotherAdapterVersion { stated: String, held: String },
}
impl HoldError {
    /// The refusal's own name, for a boundary that has to name a refusal
    /// without repeating the document: a Host's message carries this and never
    /// a path, a version or an identity the file said. Stable for the same
    /// reason the published shape is.
    pub fn name(&self) -> &'static str {
        match self {
            Self::UnknownShape { .. } => "unknown_shape",
            Self::AnotherPack { .. } => "another_pack",
            Self::TheRunNamesAnotherPack { .. } => "the_run_names_another_pack",
            Self::UnknownRules { .. } => "unknown_rules",
            Self::TheSameRunTwice { .. } => "the_same_run_twice",
            Self::ARuleDidNotHold { .. } => "a_rule_did_not_hold",
            Self::TheMatricesAreNotTheSuites { .. } => "the_matrices_are_not_the_suites",
            Self::TheRunReadNoDispatch => "the_run_read_no_dispatch",
            Self::NoRun { .. } => "no_run",
            Self::AnotherAdapterVersion { .. } => "another_adapter_version",
        }
    }
}
impl fmt::Display for HoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownShape { stated } => {
                write!(
                    f,
                    "the document speaks dossier shape {stated}, not this one"
                )
            }
            Self::AnotherPack { stated, held } => {
                write!(f, "{stated} is not {held}, and a record is one pack's")
            }
            Self::TheRunNamesAnotherPack { stated, pack } => {
                write!(f, "a run names {stated} inside a record for {pack}")
            }
            Self::UnknownRules { rules } => write!(
                f,
                "the record carries {} rules this suite does not publish: {}",
                rules.len(),
                rules.join(", ")
            ),
            Self::TheSameRunTwice {
                upstream_version,
                platform,
            } => write!(
                f,
                "upstream {upstream_version} on {platform} appears in two runs, and the pair a \
                 reader looks a run up with names one"
            ),
            Self::ARuleDidNotHold { rules } => {
                write!(
                    f,
                    "the record does not show {} held: {}",
                    rules.len(),
                    rules.join(", ")
                )
            }
            Self::TheMatricesAreNotTheSuites { what } => {
                write!(f, "the {what} matrix is not the one this suite publishes")
            }
            Self::TheRunReadNoDispatch => {
                write!(
                    f,
                    "a run read no dispatch, so its matrices record no demand"
                )
            }
            Self::NoRun {
                upstream_version,
                platform,
            } => write!(
                f,
                "the pack publishes no run for upstream {upstream_version} on {platform}"
            ),
            Self::AnotherAdapterVersion { stated, held } => {
                write!(f, "that run is of adapter version {stated}, not {held}")
            }
        }
    }
}
impl std::error::Error for HoldError {}

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
    pack(&run.adapter, run.driver.as_ref())
}

/// One spelling of a pack's identity, for every refusal that has to name one:
/// the publication that found two packs in one dossier and the reader that found
/// another pack's record where this runtime's was.
fn pack(adapter: &AgentRuntimeAdapterId, driver: Option<&HarnessDriverId>) -> String {
    match driver {
        Some(driver) => format!("{}/{}", adapter.as_str(), driver.as_str()),
        None => adapter.as_str().to_owned(),
    }
}
