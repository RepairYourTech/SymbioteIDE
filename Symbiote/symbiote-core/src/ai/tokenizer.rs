//! Tokenizer abstractions

use crate::Result;

/// Tokenizer trait
pub trait Tokenizer: Send + Sync {
    fn count_tokens(&self, text: &str) -> Result<u32>;
    fn encode(&self, text: &str) -> Result<Vec<u32>>;
    fn decode(&self, tokens: &[u32]) -> Result<String>;
}
