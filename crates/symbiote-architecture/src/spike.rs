//! This repository's own spike contracts (#173): the shape a contract has,
//! the document it is committed in, the runs that would settle the choice it
//! belongs to, and every rule those runs must meet.
//!
//! A contract is predeclared — hypothesis, workload, applicable platforms,
//! thresholds, stop conditions and cleanup — so nothing can choose its bar after
//! measuring. The bar itself is not the contract's to state: the choice's own
//! record names the section of its accepted text that says what a proof of that
//! choice must do, and the contract answers every clause of it — so it can
//! neither pick a smaller bar out of the record nor carry one its choice did not
//! accept. [`Contracts`] holds the committed document; [`Results`] holds the
//! runs that would settle it, and each [`Run`] says which platform and version it
//! exercised, on what hardware and revision, which of the contract's obligations
//! it actually ran, what it measured, which raw artifacts it published and which
//! failures it saw — and every platform the contract applies to is either
//! measured by one of its runs or declared untested, with a declared untested
//! platform keeping the choice pending. A decision can publish `accepted` only
//! while a run set answering all of that stands, so the path from `investigating`
//! to `accepted` is walked through measurements rather than through a file that
//! merely asserts them.
//!
//! Every read fails rather than passes when it finds nothing.

use crate::{ContractError, Result, require, text};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub name: String,
    pub unit: String,
    pub maximum: f64,
}

/// What answers one clause of the accepted bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Answered {
    /// The clause's own words, as the accepted section states them. The clause's
    /// identity is the record's text — which the ledger pins by SHA-256 — rather
    /// than a position in a split a comma can renumber, so an ordinal cannot
    /// drift away from the clause it was written for and the words an answer
    /// claims are read back against the record rather than trusted. Whitespace
    /// the record wraps is not part of the clause: the words are compared with
    /// every run of whitespace read as one.
    pub clause: String,
    /// The obligation this contract declares that answers it, named exactly as
    /// the contract names it: present when the clause is about the workload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub obligation: Option<String>,
    /// The part of the contract that answers it, where the clause is about how
    /// the proof is run rather than what it exercises.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<Part>,
    /// The issue that owns the clause, where this contract's workload and
    /// method do not carry it. The accepted bar is wider than any one contract,
    /// and saying which clauses are not this contract's is part of answering it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elsewhere: Option<Elsewhere>,
}

/// A clause that belongs to the decision's wider proof rather than to this
/// contract's workload: the issue that owns it, and why this contract is not
/// where it is carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Elsewhere {
    /// The issue the decision names, which owns the clause.
    pub issue: u64,
    /// Why this contract does not carry it.
    pub why: String,
}

/// The parts of a contract a clause can be answered by where it is about how
/// the proof is run: the method a run follows, and the ceilings it is measured
/// against. Everything else is an obligation or another issue's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    Method,
    Measurements,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpikeContract {
    pub schema_version: u32,
    pub id: String,
    pub decision: String,
    pub hypothesis: String,
    /// The obligations a run of this contract must exercise, each tagged with
    /// the issue it belongs to. Every one of them answers a clause of the
    /// accepted bar — the section the choice names in its own record — and
    /// [`SpikeContract::answers`] says which clause each one answers.
    pub workload: Vec<String>,
    /// The platforms this contract applies to, as data rather than as a sentence:
    /// a settlement has to account for every one of them, and prose cannot be
    /// asked what it covers.
    pub applicable_platforms: Vec<String>,
    pub platform: String,
    pub hardware: String,
    pub method: String,
    pub measurements: Vec<Measurement>,
    pub stop_conditions: Vec<String>,
    pub result_artifact: String,
    pub cleanup: String,
    /// One entry per clause of the bar the choice names — a section of the
    /// choice's own accepted text, which the ledger holds rather than this
    /// contract, so a contract cannot pick the clauses it is settled against —
    /// carrying that clause's own words and saying what answers it. Whether the
    /// carrier's wording covers those words is a reader's judgement; that the
    /// words are the accepted clause's is not, so a clause answered by nothing,
    /// and words the record does not state, are both refused where the record is
    /// read. An obligation that answers nothing is refused here.
    pub answers: Vec<Answered>,
}
impl SpikeContract {
    pub fn validate(&self) -> Result<()> {
        require(self.schema_version == 1, "unsupported spike schema")?;
        require(
            [
                &self.id,
                &self.decision,
                &self.hypothesis,
                &self.platform,
                &self.hardware,
                &self.method,
                &self.result_artifact,
                &self.cleanup,
            ]
            .iter()
            .all(|s| text(s)),
            "spike metadata incomplete",
        )?;
        require(
            !self.workload.is_empty()
                && self.workload.iter().all(|s| text(s))
                && !self.stop_conditions.is_empty()
                && self.stop_conditions.iter().all(|s| text(s)),
            "representative workload and stop conditions required",
        )?;
        require(
            !self.measurements.is_empty(),
            "predeclared measurements required",
        )?;
        require(
            !self.applicable_platforms.is_empty()
                && self.applicable_platforms.iter().all(|p| text(p))
                && self
                    .applicable_platforms
                    .iter()
                    .map(|p| p.trim().to_ascii_lowercase())
                    .collect::<BTreeSet<_>>()
                    .len()
                    == self.applicable_platforms.len(),
            "the platforms this contract applies to must be named once each",
        )?;
        let mut names = BTreeSet::new();
        for m in &self.measurements {
            require(
                text(&m.name)
                    && text(&m.unit)
                    && m.maximum.is_finite()
                    && m.maximum >= 0.0
                    && names.insert(&m.name),
                "invalid or duplicate measurement threshold",
            )?;
        }
        self.answered_bar()
    }

    /// The accepted bar, answered. Every clause of the section the choice names
    /// is answered by an obligation this contract declares, by its method or its
    /// ceilings, or by the issue that owns it elsewhere; and every obligation
    /// this contract declares answers a clause, so the contract can neither
    /// declare bar its choice did not accept nor leave a clause of it to look
    /// after itself. Which clause each answer carries is the accepted record's
    /// own text, read back there rather than trusted here.
    fn answered_bar(&self) -> Result<()> {
        require(
            !self.answers.is_empty(),
            "a contract that answers no clause of the bar its choice names states nothing about it",
        )?;
        let declared: Vec<&String> = self.workload.iter().chain(&self.stop_conditions).collect();
        let mut answered: BTreeSet<&str> = BTreeSet::new();
        for answer in &self.answers {
            let carried = [
                answer.obligation.is_some(),
                answer.part.is_some(),
                answer.elsewhere.is_some(),
            ]
            .iter()
            .filter(|present| **present)
            .count();
            require(
                carried == 1,
                format!(
                    "the answer to {:?} names one obligation, one part or one issue, not {carried} of them",
                    answer.clause
                ),
            )?;
            if let Some(obligation) = &answer.obligation {
                require(
                    declared.contains(&obligation),
                    format!(
                        "{:?} is answered by {obligation:?}, which this contract does not declare",
                        answer.clause
                    ),
                )?;
                answered.insert(obligation.as_str());
            }
            if let Some(elsewhere) = &answer.elsewhere {
                require(
                    text(&elsewhere.why),
                    format!(
                        "{:?} says #{} owns it and does not say why this contract does not",
                        answer.clause, elsewhere.issue
                    ),
                )?;
            }
        }
        require(
            declared
                .iter()
                .all(|obligation| answered.contains(obligation.as_str())),
            "this contract declares an obligation that answers no clause of the accepted bar, so it states a bar wider than the one its decision was accepted with",
        )?;
        Ok(())
    }
}

/// The contracts this repository commits, relative to the workspace root.
pub const CONTRACTS_PATH: &str = "docs/architecture/spike-contracts.json";

/// The only contract-document schema this loader replays.
pub const CONTRACTS_SCHEMA_VERSION: u32 = 1;

/// The only result-artifact schema this loader reads.
pub const RESULTS_SCHEMA_VERSION: u32 = 1;

/// The spike contracts this repository owns. A contract is predeclared: its
/// hypothesis, workload, thresholds, stop conditions and cleanup exist before
/// any candidate is run against them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contracts {
    pub schema_version: u32,
    pub contracts: Vec<SpikeContract>,
}

impl Contracts {
    /// Read and shape-check the document, refusing a schema this loader does not
    /// replay, a document that holds no contract, and a contract the crate's own
    /// rules refuse.
    pub fn read(path: &Path) -> Result<Contracts> {
        let source = std::fs::read_to_string(path).map_err(|error| {
            ContractError(format!("spike contracts {}: {error}", path.display()))
        })?;
        let contracts: Contracts = serde_json::from_str(&source).map_err(|error| {
            let detail = moved_shape(&source).unwrap_or_else(|| error.to_string());
            ContractError(format!("spike contracts {}: {detail}", path.display()))
        })?;
        require(
            contracts.schema_version == CONTRACTS_SCHEMA_VERSION,
            "unsupported spike contracts schema: this loader replays version 1 and refuses to guess",
        )?;
        require(
            !contracts.contracts.is_empty(),
            "the contract document holds no contract, so it accounts for nothing",
        )?;
        let mut ids = BTreeSet::new();
        for contract in &contracts.contracts {
            contract.validate()?;
            require(
                ids.insert(contract.id.clone()),
                format!("{}: two contracts share this identity", contract.id),
            )?;
        }
        Ok(contracts)
    }

    /// The contract with this identity, so a decision can ask what settles it.
    pub fn contract(&self, id: &str) -> Option<&SpikeContract> {
        self.contracts.iter().find(|contract| contract.id == id)
    }

    /// Whether this document holds the contract.
    pub fn holds(&self, id: &str) -> bool {
        self.contract(id).is_some()
    }
}

/// A contract document of the shape this loader read before the bar moved, named
/// rather than reported as the field it is missing.
///
/// A contract no longer carries the section it answers: the section belongs to
/// the choice's record, which names it as `proof_section`, and what answers each
/// clause of it stayed here as `answers`. Version 1 absorbs that move rather than
/// pretending to a history — no contract document exists outside this repository
/// and only this crate reads the one in it, so there is nothing to migrate — the
/// same reconciliation the result schema recorded when its own shape changed.
fn moved_shape(source: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(source).ok()?;
    let contracts = value.get("contracts")?.as_array()?;
    if contracts
        .iter()
        .any(|contract| contract.get("obligations").is_some() && contract.get("answers").is_none())
    {
        return Some(
            "it is the shape this loader read before the bar moved — a contract carrying `obligations` and no \
             `answers`: the section moved to the choice's record as `proof_section`, and what answers each clause \
             is now the contract's `answers`, while version 1 absorbs that move because no contract document \
             exists outside this repository, so rewrite the document rather than expecting a second version"
                .to_string(),
        );
    }
    let numbered = contracts.iter().any(|contract| {
        contract
            .get("answers")
            .and_then(|answers| answers.as_array())
            .is_some_and(|answers| {
                answers.iter().any(|answer| {
                    answer
                        .get("clause")
                        .is_some_and(|clause| !clause.is_string())
                })
            })
    });
    numbered.then(|| {
        "it is the shape this loader read before a clause was named by its own words — answers carrying a \
         `clause` number: a clause is now identified by the accepted section's words, which the ledger pins by \
         SHA-256, so an answer names the clause it answers rather than a position in a split the record's \
         punctuation can renumber, while version 1 absorbs that move for the same reason the section's move was \
         absorbed, so rewrite each answer's `clause` to carry that clause's own words"
            .to_string()
    })
}

/// How one run ended against the contract it was measured under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Every predeclared measurement was taken and met.
    WithinThresholds,
    /// A stop condition ended the run, so the contract does not settle.
    StopConditionTriggered,
}

/// One predeclared measurement as a run observed it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub measurement: String,
    pub observed: f64,
}

/// One raw artifact a run published, content-addressed so it can be re-checked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cited {
    pub artifact: String,
    pub sha256: String,
}

/// One run of a contract on one platform: what it ran, where, on what, what it
/// measured, what it published and what failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// The platform this run exercised. A run on one platform says nothing about
    /// the others: the file has to declare which of the contract's platforms it
    /// left alone, and declaring one keeps the choice from settling.
    #[serde(default)]
    pub platform: String,
    /// The platform's version, so the figures belong to a stated stack.
    #[serde(default)]
    pub version: String,
    /// The machine the run happened on, so its figures carry a reference
    /// configuration.
    #[serde(default)]
    pub hardware: String,
    /// The revision the run was built from.
    #[serde(default)]
    pub commit: String,
    /// The contract obligations this run exercised, named exactly as the
    /// contract names them, so a run cannot attest a workload the contract never
    /// declared and cannot leave one it did silent.
    #[serde(default)]
    pub exercised: Vec<String>,
    pub outcome: Outcome,
    /// Present exactly when a stop condition ended the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_condition: Option<String>,
    /// The failures this run saw, named. An empty list is a claim that it saw
    /// none, which a run that stopped may not make.
    #[serde(default)]
    pub failures: Vec<String>,
    pub observations: Vec<Observation>,
    /// The raw data and failure cases this run published.
    #[serde(default)]
    pub artifacts: Vec<Cited>,
}

impl Run {
    /// Every requirement the contract makes of this run, in order: what the run
    /// says about itself, what it ran, what it published, how it ended and what
    /// it measured. These hold only where a choice is being settled: a partial
    /// run may commit less.
    pub fn unmet(&self, contract: &SpikeContract) -> Vec<String> {
        let mut unmet = Vec::new();
        unmet.extend(self.identity_unmet());
        unmet.extend(self.workload_unmet(contract));
        unmet.extend(self.published_unmet());
        unmet.extend(self.outcome_unmet(contract));
        unmet.extend(self.observations_unmet(contract));
        unmet
    }

    /// The name a refusal gives this run: the platform it exercised, or "a run"
    /// while it names none.
    fn label(&self) -> String {
        let platform = self.platform.trim();
        if platform.is_empty() {
            "a run".to_string()
        } else {
            format!("the run on {platform}")
        }
    }

    /// Where and on what this run happened, as a candidate's figures need it.
    fn identity_unmet(&self) -> Vec<String> {
        let mut unmet = Vec::new();
        let label = self.label();
        if self.platform.trim().is_empty() {
            unmet.push("a run names no platform, so nothing says where it was measured".into());
        }
        if !text(&self.version) {
            unmet.push(format!(
                "{label} records no platform version, so its platform is unpinned"
            ));
        }
        if !text(&self.hardware) {
            unmet.push(format!(
                "{label} records no hardware, so its figures carry no reference configuration"
            ));
        }
        if !revision(&self.commit) {
            unmet.push(format!(
                "{label} names no revision: {:?} is not a commit hash",
                self.commit
            ));
        }
        unmet
    }

    /// The workload this run attests is the contract's workload: every declared
    /// obligation, and nothing the contract never declared.
    fn workload_unmet(&self, contract: &SpikeContract) -> Vec<String> {
        let mut unmet = Vec::new();
        let label = self.label();
        let exercised: BTreeSet<&str> = self.exercised.iter().map(String::as_str).collect();
        for entry in &exercised {
            if !contract
                .workload
                .iter()
                .any(|obligation| obligation == entry)
            {
                unmet.push(format!(
                    "{label} attests {entry:?}, which the contract's workload never declares"
                ));
            }
        }
        for obligation in &contract.workload {
            if !exercised.contains(obligation.as_str()) {
                unmet.push(format!(
                    "{label} does not attest {obligation:?}, which the contract's workload declares"
                ));
            }
        }
        unmet
    }

    /// What stands behind the figures: published raw data, and failures that say
    /// what happened.
    fn published_unmet(&self) -> Vec<String> {
        let mut unmet = Vec::new();
        let label = self.label();
        if self.artifacts.is_empty() {
            unmet.push(format!(
                "{label} publishes no raw artifact, so nothing stands behind its figures"
            ));
        }
        for failure in &self.failures {
            if !text(failure) {
                unmet.push(format!(
                    "{label} lists a failure with nothing said about it"
                ));
            }
        }
        unmet
    }

    /// How the run ended, against the stop conditions the contract declares.
    fn outcome_unmet(&self, contract: &SpikeContract) -> Vec<String> {
        let mut unmet = Vec::new();
        let label = self.label();
        match self.outcome {
            Outcome::WithinThresholds => {
                if let Some(condition) = &self.stop_condition {
                    unmet.push(format!(
                        "{label} met the thresholds and still records the stop condition {condition:?}"
                    ));
                }
            }
            Outcome::StopConditionTriggered => {
                match self
                    .stop_condition
                    .as_deref()
                    .filter(|condition| text(condition))
                {
                    None => unmet.push(format!(
                        "{label} stopped without naming the stop condition that ended it"
                    )),
                    Some(condition) => {
                        unmet.push(format!(
                            "{label} stopped on {condition:?}, so the contract does not settle the choice"
                        ));
                        if !contract
                            .stop_conditions
                            .iter()
                            .any(|declared| declared == condition)
                        {
                            unmet.push(format!(
                                "{label} stops on {condition:?}, which the contract's stop conditions never declare"
                            ));
                        }
                        if !self.failures.iter().any(|failure| failure == condition) {
                            unmet.push(format!(
                                "{label} stopped on {condition:?} and does not record it among its failures"
                            ));
                        }
                    }
                }
            }
        }
        unmet
    }

    /// Every predeclared measurement, once, inside its threshold — and nothing
    /// the contract never predeclared.
    fn observations_unmet(&self, contract: &SpikeContract) -> Vec<String> {
        let mut unmet = Vec::new();
        let label = self.label();
        let mut observed: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for observation in &self.observations {
            observed
                .entry(observation.measurement.as_str())
                .or_default()
                .push(observation.observed);
        }
        for measurement in &contract.measurements {
            match observed.remove(measurement.name.as_str()).as_deref() {
                None => unmet.push(format!(
                    "{label} records no observation for {}, which the contract predeclares",
                    measurement.name
                )),
                Some([value]) => {
                    if !value.is_finite() {
                        unmet.push(format!(
                            "{label} records a {} observation that is not a finite number",
                            measurement.name
                        ));
                    } else if *value > measurement.maximum {
                        unmet.push(format!(
                            "{label} observed {} at {value:?} {}, above the predeclared maximum of {:?}",
                            measurement.name, measurement.unit, measurement.maximum
                        ));
                    }
                }
                Some(values) => unmet.push(format!(
                    "{label} records the predeclared measurement {} {} times",
                    measurement.name,
                    values.len()
                )),
            }
        }
        for name in observed.keys() {
            unmet.push(format!(
                "{label} observes {name}, which the contract does not predeclare"
            ));
        }
        unmet
    }
}

/// What would settle a choice: the runs measured against one contract, and the
/// platforms they left alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Results {
    pub schema_version: u32,
    /// The contract identity these runs measured.
    pub contract: String,
    /// The contract's fingerprint when the runs happened, so a threshold cannot
    /// move under results measured against it.
    pub contract_sha256: String,
    /// Platforms the run set did not exercise. An untested platform stays
    /// pending, so a non-empty list keeps the choice from settling.
    #[serde(default)]
    pub untested_platforms: Vec<String>,
    #[serde(default)]
    pub runs: Vec<Run>,
}

impl Results {
    /// Read a result artifact. A result that cannot be read is an error rather
    /// than an empty one, because a result with nothing in it settles nothing.
    pub fn read(path: &Path) -> Result<Results> {
        let source = std::fs::read_to_string(path)
            .map_err(|error| ContractError(format!("spike results {}: {error}", path.display())))?;
        let results: Results = serde_json::from_str(&source)
            .map_err(|error| ContractError(format!("spike results {}: {error}", path.display())))?;
        require(
            results.schema_version == RESULTS_SCHEMA_VERSION,
            "unsupported spike results schema: this loader reads version 1 and refuses to guess",
        )?;
        Ok(results)
    }

    /// Whether these results are runs of *this* contract — the part that holds
    /// whether or not the choice is decided, so a partial run is still tied to
    /// the contract and thresholds it was measured against.
    pub fn identity_unmet(&self, contract: &SpikeContract, fingerprint: &str) -> Vec<String> {
        let mut unmet = Vec::new();
        if self.contract != contract.id {
            unmet.push(format!(
                "the result measured {}, not {}",
                self.contract, contract.id
            ));
        }
        if self.contract_sha256 != fingerprint {
            unmet.push(format!(
                "the result records contract {} and the committed contract fingerprints {fingerprint}, so the contract it was measured against is not the one committed now",
                self.contract_sha256
            ));
        }
        unmet
    }

    /// Every requirement this contract makes of a run set that would settle the
    /// choice, in order. An empty result is the only one that settles it.
    pub fn unmet(&self, contract: &SpikeContract, fingerprint: &str) -> Vec<String> {
        let mut unmet = self.identity_unmet(contract, fingerprint);
        if self.runs.is_empty() {
            unmet.push("the result records no run, so nothing has been measured".into());
        }
        for run in &self.runs {
            let platform = run.platform.trim();
            if !platform.is_empty()
                && self
                    .untested_platforms
                    .iter()
                    .any(|untested| untested.trim().eq_ignore_ascii_case(platform))
            {
                unmet.push(format!(
                    "the result lists {platform}, the platform that run exercised, as untested"
                ));
            }
            unmet.extend(run.unmet(contract));
        }
        if !self.untested_platforms.is_empty() {
            unmet.push(format!(
                "{} platform(s) were not exercised, and an untested platform stays pending: {}",
                self.untested_platforms.len(),
                self.untested_platforms.join(", ")
            ));
        }
        unmet.extend(self.coverage_unmet(contract));
        unmet
    }

    /// Every platform the contract applies to is accounted for: measured by a
    /// run, or declared untested — and an untested declaration is only about a
    /// platform the contract applies to. Coverage is the contract's to declare,
    /// so a result cannot settle a choice by saying nothing about a platform: the
    /// list of what it left alone has to be complete, which is what makes the
    /// choice pending rather than settled where a platform was not exercised.
    fn coverage_unmet(&self, contract: &SpikeContract) -> Vec<String> {
        let mut unmet = Vec::new();
        if self.runs.is_empty() {
            // A result that records no run is refused for exactly that, so the
            // platforms it therefore covers are not counted a second time.
            return unmet;
        }
        let measured: BTreeSet<String> = self
            .runs
            .iter()
            .map(|run| platform_key(&run.platform))
            .filter(|platform| !platform.is_empty())
            .collect();
        let untested: BTreeSet<String> = self
            .untested_platforms
            .iter()
            .map(|platform| platform_key(platform))
            .filter(|platform| !platform.is_empty())
            .collect();
        let applicable: BTreeSet<String> = contract
            .applicable_platforms
            .iter()
            .map(|platform| platform_key(platform))
            .collect();
        for platform in untested.difference(&applicable) {
            unmet.push(format!(
                "the result declares {platform:?} untested, which the contract does not apply to"
            ));
        }
        for platform in applicable
            .difference(&untested)
            .filter(|p| !measured.contains(*p))
        {
            unmet.push(format!(
                "the result leaves {platform:?} neither measured nor declared untested, so nothing says whether the contract's applicability was covered"
            ));
        }
        unmet
    }
}

/// One platform, as a result names it: trimmed and case-insensitive, so a
/// contract's "Linux X11" and a run's "linux-x11" are the same platform.
fn platform_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

/// Whether a string names a revision: what git writes, abbreviated or full.
fn revision(value: &str) -> bool {
    let value = value.trim();
    (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The text of one section of a record: what follows its `## ` heading, up to
/// the next heading. `None` where the record does not state that section, so a
/// bar named from a heading nobody wrote is refused rather than read as empty.
pub fn section(text: &str, heading: &str) -> Option<String> {
    let mut inside = false;
    let mut body = String::new();
    for line in text.lines() {
        if let Some(name) = line.strip_prefix("## ") {
            inside = name.trim() == heading.trim();
            continue;
        }
        if inside {
            body.push_str(line);
            body.push('\n');
        }
    }
    let body = body.trim();
    (!body.is_empty()).then(|| body.to_string())
}

/// The clauses one section states: a sentence and a semicolon each end one,
/// because that is how the accepted text enumerates what a proof must do. The
/// split is the record's punctuation and nothing else — a clause is the text
/// itself, so nothing here can quietly decide the bar is smaller.
pub fn clauses(section: &str) -> Vec<String> {
    normalize(section)
        .split_inclusive('.')
        .flat_map(|sentence| sentence.split(';'))
        .map(normalize)
        .filter(|clause| !clause.is_empty())
        .collect()
}

/// Text as one line of words: every run of whitespace read as one space, and no
/// leading or trailing space. A clause's identity is its words, so how the
/// record wraps them and how a contract document wraps them are the same text.
pub fn normalize(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The fingerprint of a contract: the SHA-256 of its own canonical JSON, so a
/// result is tied to the thresholds it was measured against and adding another
/// contract to the document does not move an existing result.
pub fn fingerprint(contract: &SpikeContract) -> String {
    let bytes = serde_json::to_vec(contract).unwrap_or_default();
    let mut digest = Sha256::new();
    digest.update(&bytes);
    format!("{:x}", digest.finalize())
}
