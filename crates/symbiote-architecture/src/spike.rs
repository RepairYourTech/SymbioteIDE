//! This repository's own spike contract (#173), and the result artifact that
//! would settle the choice it belongs to.
//!
//! [`SpikeContract`] already existed in this crate with no contract to read:
//! #38's proof obligations lived in prose, so nothing could say what the shell
//! choice would be settled *by*. [`Contracts`] holds the committed contract and
//! [`Results`] holds what a run observed, so a decision can publish `accepted`
//! only while a complete, in-threshold run stands — a predeclared bar rather
//! than a bar chosen after the measurement.
//!
//! Every read fails rather than passes when it finds nothing.

use crate::{ContractError, Result, SpikeContract, require};
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

/// How a run ended against the contract it was measured under.
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

/// What one run of a contract observed: against which contract, with what
/// outcome, and which raw artifacts stand behind it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Results {
    pub schema_version: u32,
    /// The contract identity this run measured.
    pub contract: String,
    /// The contract's fingerprint when the run happened, so a threshold cannot
    /// move under a result measured against it.
    pub contract_sha256: String,
    pub outcome: Outcome,
    /// Present exactly when a stop condition ended the run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_condition: Option<String>,
    /// Platforms the run did not exercise. An untested platform stays pending,
    /// so a non-empty list keeps the choice from settling.
    #[serde(default)]
    pub untested_platforms: Vec<String>,
    pub observations: Vec<Observation>,
    /// The raw data and failure cases this run published.
    #[serde(default)]
    pub artifacts: Vec<Cited>,
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

    /// Whether this result is a run of *this* contract — the part that holds
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

    /// Every requirement this contract makes of a run that would settle the
    /// choice, in order. An empty result is the only one that settles it.
    pub fn unmet(&self, contract: &SpikeContract, fingerprint: &str) -> Vec<String> {
        let mut unmet = self.identity_unmet(contract, fingerprint);
        match self.outcome {
            Outcome::WithinThresholds => {
                if let Some(condition) = &self.stop_condition {
                    unmet.push(format!(
                        "the run met the thresholds and still records the stop condition {condition:?}"
                    ));
                }
            }
            Outcome::StopConditionTriggered => unmet.push(format!(
                "the run ended on a stop condition{}, so the contract does not settle the choice",
                self.stop_condition
                    .as_ref()
                    .map_or_else(String::new, |condition| format!(": {condition}"))
            )),
        }
        if !self.untested_platforms.is_empty() {
            unmet.push(format!(
                "{} platform(s) were not exercised, and an untested platform stays pending: {}",
                self.untested_platforms.len(),
                self.untested_platforms.join(", ")
            ));
        }
        let mut observed: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for observation in &self.observations {
            observed
                .entry(observation.measurement.as_str())
                .or_default()
                .push(observation.observed);
        }
        for measurement in &contract.measurements {
            let values = observed.remove(measurement.name.as_str());
            match values.as_deref() {
                None => unmet.push(format!(
                    "the contract predeclares {} and the run recorded no observation for it",
                    measurement.name
                )),
                Some([value]) => {
                    if !value.is_finite() {
                        unmet.push(format!(
                            "the {} observation is not a finite number",
                            measurement.name
                        ));
                    } else if *value > measurement.maximum {
                        unmet.push(format!(
                            "{} observed {value:?} {}, above the predeclared maximum of {:?}",
                            measurement.name, measurement.unit, measurement.maximum
                        ));
                    }
                }
                Some(values) => unmet.push(format!(
                    "the contract predeclares {} once and the run observed it {} times",
                    measurement.name,
                    values.len()
                )),
            }
        }
        for name in observed.keys() {
            unmet.push(format!(
                "the run observes {name}, which the contract does not predeclare"
            ));
        }
        unmet
    }
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
