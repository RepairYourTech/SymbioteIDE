//! This repository's own spike contract (#173), and the result artifact that
//! would settle the choice it belongs to.
//!
//! [`SpikeContract`] already existed in this crate with no contract to read:
//! #38's proof obligations lived in prose, so nothing could say what the shell
//! choice would be settled *by*, and nothing stopped a settlement resting on a
//! file that named no run at all. [`Contracts`] holds the committed contract;
//! [`Results`] holds the runs that would settle it, and each [`Run`] says which
//! platform and version it exercised, on what hardware and revision, which of
//! the contract's obligations it actually ran, what it measured, which raw
//! artifacts it published and which failures it saw. A decision can publish
//! `accepted` only while a run set answering all of that stands, so the path
//! from `investigating` to `accepted` is walked through measurements rather than
//! through a file that merely asserts them.
//!
//! Every read fails rather than passes when it finds nothing.

use crate::{ContractError, Result, SpikeContract, require, text};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

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
            ContractError(format!("spike contracts {}: {error}", path.display()))
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
    /// the others; the platforms the run set leaves alone are the file's
    /// business.
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
    /// Every requirement the contract makes of this run, in order. These hold
    /// only where a choice is being settled: a partial run may commit less.
    pub fn unmet(&self, contract: &SpikeContract) -> Vec<String> {
        let mut unmet = Vec::new();
        let platform = self.platform.trim();
        let label = if platform.is_empty() {
            "a run".to_string()
        } else {
            format!("the run on {platform}")
        };
        if platform.is_empty() {
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
                "the result records contract {} and the committed contract fingerprints {fingerprint}, so its thresholds have moved since the run",
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
        unmet
    }
}

/// Whether a string names a revision: what git writes, abbreviated or full.
fn revision(value: &str) -> bool {
    let value = value.trim();
    (7..=64).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
