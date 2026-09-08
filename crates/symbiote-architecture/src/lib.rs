//! Versioned decision/proof contracts (#173). These are in-memory policy
//! mechanisms; the future Host must authenticate authority and evidence inputs.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError(pub String);
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ContractError {}
type Result<T> = std::result::Result<T, ContractError>;
fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(ContractError(message.into()))
    }
}
fn text(value: &str) -> bool {
    !value.trim().is_empty()
}
fn next(revision: u64) -> Result<u64> {
    revision
        .checked_add(1)
        .ok_or_else(|| ContractError("revision exhausted".into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DecisionState {
    Proposed,
    Investigating,
    Accepted,
    Superseded,
    Rejected,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    Client,
    DelegatedArchitect,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionPin {
    pub id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub artifact: String,
    pub sha256: String,
    pub method: String,
    pub observed_at: u64,
    pub reviewer: String,
}
impl Evidence {
    fn validate(&self, now: u64) -> Result<()> {
        require(
            text(&self.artifact) && text(&self.method) && text(&self.reviewer),
            "evidence requires artifact, method and reviewer",
        )?;
        require(
            self.sha256.len() == 64 && self.sha256.bytes().all(|c| c.is_ascii_hexdigit()),
            "evidence requires SHA-256",
        )?;
        require(
            self.observed_at <= now,
            "evidence observation is in the future",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub schema_version: u32,
    pub id: String,
    pub revision: u64,
    pub state: DecisionState,
    pub title: String,
    pub constraints: Vec<String>,
    pub alternatives: Vec<String>,
    pub consequences: Vec<String>,
    pub security: String,
    pub reversal: String,
    pub high_reversal_cost: bool,
    pub issue_refs: Vec<u64>,
    pub requirement_refs: Vec<String>,
    pub graph_refs: Vec<String>,
    pub deployment_refs: Vec<String>,
    pub compatibility: Vec<VersionPin>,
    pub evidence: Vec<Evidence>,
    pub supersedes: Option<String>,
}
impl Decision {
    fn validate(&self) -> Result<()> {
        require(
            self.schema_version == 1 && self.revision > 0,
            "unsupported decision schema or zero revision",
        )?;
        require(
            [&self.id, &self.title, &self.security, &self.reversal]
                .iter()
                .all(|s| text(s)),
            "decision requires identity, title, security and reversal",
        )?;
        for list in [&self.constraints, &self.alternatives, &self.consequences] {
            require(
                !list.is_empty() && list.iter().all(|s| text(s)),
                "constraints, alternatives and consequences must be explicit",
            )?;
        }
        require(
            !self.issue_refs.is_empty() && self.issue_refs.iter().all(|n| *n > 0),
            "canonical issue ownership required",
        )?;
        require(
            !self.requirement_refs.is_empty() && self.requirement_refs.iter().all(|s| text(s)),
            "requirement links required",
        )?;
        let mut ids = BTreeSet::new();
        for pin in &self.compatibility {
            require(
                text(&pin.id) && pin.revision > 0 && ids.insert(&pin.id),
                "invalid or duplicate compatibility pin",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Verified,
    Unknown,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityFact {
    pub id: String,
    pub revision: u64,
    pub tested_version: String,
    pub sources: Vec<String>,
    pub observed_at: u64,
    pub expires_at: u64,
    pub status: FactStatus,
    pub native_path: String,
    pub precedence: String,
    pub trust_behavior: String,
    pub unknowns: Vec<String>,
}
impl CompatibilityFact {
    fn validate(&self) -> Result<()> {
        require(
            self.revision > 0
                && [
                    &self.id,
                    &self.tested_version,
                    &self.native_path,
                    &self.precedence,
                    &self.trust_behavior,
                ]
                .iter()
                .all(|s| text(s)),
            "compatibility fact metadata incomplete",
        )?;
        require(
            !self.sources.is_empty() && self.sources.iter().all(|s| s.starts_with("https://")),
            "compatibility fact requires authoritative HTTPS source references",
        )?;
        require(
            self.expires_at > self.observed_at,
            "fact expiry must follow observation",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImplementationPins {
    pub artifact: String,
    pub decisions: Vec<VersionPin>,
    pub compatibility: Vec<VersionPin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    pub actor: String,
    pub authority: Authority,
    pub now: u64,
    /// Explicit client authorization replaces measurement only for this choice.
    pub client_exception: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Settled,
    Provisional,
    Blocked,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GateResult {
    pub status: GateStatus,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DecisionRegistry {
    decisions: BTreeMap<String, Decision>,
    facts: BTreeMap<String, CompatibilityFact>,
    artifacts: BTreeMap<String, ImplementationPins>,
    approvals: BTreeMap<String, Acceptance>,
}
impl DecisionRegistry {
    pub fn decision(&self, id: &str) -> Option<&Decision> {
        self.decisions.get(id)
    }
    pub fn approval(&self, id: &str) -> Option<&Acceptance> {
        self.approvals.get(id)
    }

    pub fn propose(&mut self, decision: Decision) -> Result<()> {
        decision.validate()?;
        require(
            decision.state == DecisionState::Proposed && decision.revision == 1,
            "new decisions must be proposed at revision 1",
        )?;
        require(
            !self.decisions.contains_key(&decision.id),
            "decision identity already exists",
        )?;
        if let Some(old) = &decision.supersedes {
            require(
                self.decisions
                    .get(old)
                    .is_some_and(|d| d.state == DecisionState::Accepted),
                "supersession requires an accepted predecessor",
            )?;
        }
        self.decisions.insert(decision.id.clone(), decision);
        Ok(())
    }

    pub fn revise_draft(
        &mut self,
        expected: u64,
        mut replacement: Decision,
    ) -> Result<Vec<String>> {
        let current = self
            .decisions
            .get(&replacement.id)
            .ok_or_else(|| ContractError("unknown decision".into()))?;
        require(current.revision == expected, "decision revision conflict")?;
        require(
            matches!(
                current.state,
                DecisionState::Proposed | DecisionState::Investigating
            ),
            "accepted and terminal decisions are immutable; propose superseding lineage",
        )?;
        require(
            matches!(
                replacement.state,
                DecisionState::Proposed
                    | DecisionState::Investigating
                    | DecisionState::Rejected
                    | DecisionState::Retired
            ),
            "draft revision cannot accept or supersede",
        )?;
        require(
            replacement.supersedes == current.supersedes,
            "supersession lineage is immutable",
        )?;
        replacement.revision = next(expected)?;
        replacement.validate()?;
        let affected = self.affected_by_decision(&replacement.id);
        self.decisions.insert(replacement.id.clone(), replacement);
        Ok(affected)
    }

    /// All validation precedes mutation. Acceptance also supersedes the old
    /// record atomically in this in-memory boundary, without changing its content.
    pub fn accept(&mut self, id: &str, expected: u64, approval: Acceptance) -> Result<Vec<String>> {
        let current = self
            .decisions
            .get(id)
            .ok_or_else(|| ContractError("unknown decision".into()))?;
        require(current.revision == expected, "decision revision conflict")?;
        require(
            matches!(
                current.state,
                DecisionState::Proposed | DecisionState::Investigating
            ),
            "decision is not a draft",
        )?;
        require(
            approval.authority != Authority::Worker && text(&approval.actor),
            "worker cannot authorize architecture",
        )?;
        if let Some(exception) = &approval.client_exception {
            require(
                approval.authority == Authority::Client && text(exception),
                "only explicit client authority can waive measurement",
            )?;
        }
        require(
            !current.high_reversal_cost
                || !current.evidence.is_empty()
                || approval.client_exception.is_some(),
            "high reversal cost requires measured evidence or explicit client exception",
        )?;
        for evidence in &current.evidence {
            evidence.validate(approval.now)?;
        }
        for fact in &current.compatibility {
            self.check_fact(fact, approval.now)?;
        }
        let revision = next(expected)?;
        let old_id = current.supersedes.clone();
        let old_revision = if let Some(old_id) = &old_id {
            let old = self
                .decisions
                .get(old_id)
                .ok_or_else(|| ContractError("unknown predecessor".into()))?;
            require(
                old.state == DecisionState::Accepted,
                "predecessor is no longer accepted",
            )?;
            Some(next(old.revision)?)
        } else {
            None
        };
        let mut affected: BTreeSet<String> = self.affected_by_decision(id).into_iter().collect();
        if let Some(old) = &old_id {
            affected.extend(self.affected_by_decision(old));
        }
        let decision = self.decisions.get_mut(id).expect("validated decision");
        decision.state = DecisionState::Accepted;
        decision.revision = revision;
        if let (Some(old), Some(revision)) = (old_id, old_revision) {
            let predecessor = self.decisions.get_mut(&old).expect("validated predecessor");
            predecessor.state = DecisionState::Superseded;
            predecessor.revision = revision;
        }
        self.approvals.insert(id.into(), approval);
        Ok(affected.into_iter().collect())
    }

    pub fn put_fact(
        &mut self,
        expected: Option<u64>,
        fact: CompatibilityFact,
    ) -> Result<Vec<String>> {
        fact.validate()?;
        match self.facts.get(&fact.id) {
            None => require(
                expected.is_none() && fact.revision == 1,
                "new fact must start at revision 1",
            )?,
            Some(old) => {
                require(expected == Some(old.revision), "fact revision conflict")?;
                require(
                    fact.revision == next(old.revision)?,
                    "fact revision must advance exactly once",
                )?;
            }
        }
        let affected = self.affected_by_fact(&fact.id);
        self.facts.insert(fact.id.clone(), fact);
        Ok(affected)
    }

    pub fn register_implementation(&mut self, pins: ImplementationPins) -> Result<()> {
        require(
            text(&pins.artifact) && !pins.decisions.is_empty(),
            "implementation requires artifact and decision pins",
        )?;
        require(
            !self.artifacts.contains_key(&pins.artifact),
            "artifact pins are immutable; use a new artifact revision",
        )?;
        for pin in pins.decisions.iter().chain(&pins.compatibility) {
            require(text(&pin.id) && pin.revision > 0, "invalid version pin")?;
        }
        self.artifacts.insert(pins.artifact.clone(), pins);
        Ok(())
    }

    fn check_fact(&self, pin: &VersionPin, now: u64) -> Result<()> {
        let fact = self
            .facts
            .get(&pin.id)
            .ok_or_else(|| ContractError(format!("unknown compatibility fact {}", pin.id)))?;
        require(
            fact.revision == pin.revision,
            "compatibility revision changed",
        )?;
        require(
            fact.status == FactStatus::Verified,
            "compatibility unknown or revoked",
        )?;
        require(
            fact.observed_at <= now && now < fact.expires_at,
            "compatibility evidence is stale or future-dated",
        )
    }

    pub fn gate(&self, artifact: &str, now: u64) -> GateResult {
        let Some(pins) = self.artifacts.get(artifact) else {
            return GateResult {
                status: GateStatus::Blocked,
                reasons: vec!["unknown implementation artifact".into()],
            };
        };
        let mut result = GateResult {
            status: GateStatus::Settled,
            reasons: vec![],
        };
        for pin in &pins.decisions {
            match self.decisions.get(&pin.id) {
                None => {
                    result.status = GateStatus::Blocked;
                    result.reasons.push(format!("unknown decision {}", pin.id));
                }
                Some(d)
                    if d.revision != pin.revision
                        || matches!(
                            d.state,
                            DecisionState::Superseded | DecisionState::Retired
                        ) =>
                {
                    if result.status != GateStatus::Blocked {
                        result.status = GateStatus::Stale;
                    }
                    result
                        .reasons
                        .push(format!("decision {} changed or retired", pin.id));
                }
                Some(d) if d.state != DecisionState::Accepted => {
                    if d.state == DecisionState::Rejected {
                        result.status = GateStatus::Blocked;
                    } else if result.status == GateStatus::Settled {
                        result.status = GateStatus::Provisional;
                    }
                    result
                        .reasons
                        .push(format!("decision {} is not accepted", pin.id));
                }
                Some(d) => {
                    for fact in &d.compatibility {
                        if let Err(e) = self.check_fact(fact, now) {
                            if result.status != GateStatus::Blocked {
                                result.status = GateStatus::Stale;
                            }
                            result.reasons.push(format!("{}: {e}", fact.id));
                        }
                    }
                }
            }
        }
        for fact in &pins.compatibility {
            if let Err(e) = self.check_fact(fact, now) {
                if result.status != GateStatus::Blocked {
                    result.status = GateStatus::Stale;
                }
                result.reasons.push(format!("{}: {e}", fact.id));
            }
        }
        result
    }

    pub fn require_ready(&self, artifact: &str, now: u64) -> Result<()> {
        let result = self.gate(artifact, now);
        require(
            result.status == GateStatus::Settled,
            &result.reasons.join("; "),
        )
    }

    fn affected_by_decision(&self, id: &str) -> Vec<String> {
        self.artifacts
            .values()
            .filter(|a| a.decisions.iter().any(|p| p.id == id))
            .map(|a| a.artifact.clone())
            .collect()
    }
    fn affected_by_fact(&self, id: &str) -> Vec<String> {
        self.artifacts
            .values()
            .filter(|a| {
                a.compatibility.iter().any(|p| p.id == id)
                    || a.decisions.iter().any(|p| {
                        self.decisions
                            .get(&p.id)
                            .is_some_and(|d| d.compatibility.iter().any(|f| f.id == id))
                    })
            })
            .map(|a| a.artifact.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub name: String,
    pub unit: String,
    pub maximum: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpikeContract {
    pub schema_version: u32,
    pub id: String,
    pub decision: String,
    pub hypothesis: String,
    pub workload: Vec<String>,
    pub platform: String,
    pub hardware: String,
    pub method: String,
    pub measurements: Vec<Measurement>,
    pub stop_conditions: Vec<String>,
    pub result_artifact: String,
    pub cleanup: String,
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
        Ok(())
    }
}
