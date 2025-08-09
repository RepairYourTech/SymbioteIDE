//! Core engine

use crate::Result;

/// Core engine for Symbiote
pub struct SymbioteEngine {
    initialized: bool,
}

impl SymbioteEngine {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        self.initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Default for SymbioteEngine {
    fn default() -> Self {
        Self::new()
    }
}
