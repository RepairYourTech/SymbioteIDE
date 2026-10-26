//! The versioned capability catalog: what the operator's connection offers,
//! held as a document the Host can validate before inference rather than as
//! claims the adapter discovers by failing.
//!
//! A catalog is a versioned document — `catalog_version` plus one entry per model
//! — and `authorize` is the gate the transport consults before its first byte:
//! the model must be one the document names, the requested reasoning effort must
//! be one that model supports, the input estimate must fit the model's context
//! window, the output budget must fit its output bound, and tools must be
//! requested only of a model that declares them. Every one of those refusals is
//! pre-inference by construction: nothing here looks at the network, so a request
//! this document refuses is a request the provider never sees. That is the shape
//! #471 asks for — "invalid effort or oversized context is rejected before
//! inference rather than silently coerced" — and the reason coercion is absent on
//! purpose: an effort the document does not name is refused, not downgraded to
//! the nearest one, because a silently coerced effort is a run that cannot say
//! what it asked the model for.
//!
//! The document is versioned for the changed-catalog case: a connection whose
//! models moved loads the new document under a new version, and a request built
//! against the old one fails by name (`ModelMismatch`) rather than by drifting
//! into whatever the new document happens to allow. Discovery can fill this
//! document in; nothing in it is believed without the operator (or the discovery
//! evidence chain) standing behind it, which is the AUTH-01 boundary this crate
//! stays inside.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use symbiote_domain::ModelId;
use symbiote_runtime_sdk::provider::{ProviderError, ProviderRequest, ReasoningEffort};

/// The catalog document's own contract version, independent of the request
/// envelope's: a catalog the reader cannot hold is refused, not reinterpreted.
pub const CATALOG_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogModel {
    pub model_id: ModelId,
    pub context_window_tokens: u64,
    pub max_output_tokens: u64,
    pub supported_reasoning: BTreeSet<ReasoningEffort>,
    pub tools: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCatalog {
    pub catalog_version: u32,
    pub models: Vec<CatalogModel>,
}

impl ProviderCatalog {
    /// A catalog the reader cannot hold is refused before it can refuse
    /// anything: the version must be one this reader speaks, and the entries
    /// must be whole (non-zero bounds) and one entry per model.
    pub fn parse(bytes: &[u8]) -> Result<Self, ProviderError> {
        let catalog: Self =
            serde_json::from_slice(bytes).map_err(|_| ProviderError::InvalidEnvelope)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), ProviderError> {
        if self.catalog_version != CATALOG_CONTRACT_VERSION || self.models.is_empty() {
            return Err(ProviderError::UnsupportedVersion);
        }
        let mut seen = BTreeSet::new();
        for model in &self.models {
            if model.context_window_tokens == 0
                || model.max_output_tokens == 0
                || model.max_output_tokens > model.context_window_tokens
                || !seen.insert(model.model_id.clone())
            {
                return Err(ProviderError::InvalidDescriptor);
            }
        }
        Ok(())
    }

    /// Whether this catalog holds the one model the descriptor names, with
    /// bounds that at least match the descriptor's own — a catalog entry the
    /// descriptor outruns would let a request pass the catalog and fail the
    /// envelope validation after it, which is the drift this check names.
    pub fn holds(&self, model: &symbiote_runtime_sdk::provider::ModelDescriptor) -> bool {
        self.models.iter().any(|entry| {
            entry.model_id == model.id
                && entry.context_window_tokens >= model.context_window_tokens
                && entry.max_output_tokens >= model.max_output_tokens
        })
    }

    /// The pre-inference gate. Refuses, in order: a model the catalog does
    /// not hold, an output budget past the entry's bound, an input estimate
    /// past its context window, a reasoning effort the entry does not
    /// support, and tools requested of a model that declares none.
    pub fn authorize(&self, request: &ProviderRequest) -> Result<(), ProviderError> {
        let entry = self
            .models
            .iter()
            .find(|entry| entry.model_id == request.model_id)
            .ok_or(ProviderError::ModelMismatch)?;
        if request.max_output_tokens > entry.max_output_tokens {
            return Err(ProviderError::OutputLimit);
        }
        if request.input_token_budget > entry.context_window_tokens {
            return Err(ProviderError::ContextLimit);
        }
        if let Some(effort) = &request.reasoning {
            if !entry.supported_reasoning.contains(effort) {
                return Err(ProviderError::UnsupportedCapability);
            }
        }
        if !entry.tools && !request.tools.is_empty() {
            return Err(ProviderError::UnsupportedCapability);
        }
        Ok(())
    }
}
