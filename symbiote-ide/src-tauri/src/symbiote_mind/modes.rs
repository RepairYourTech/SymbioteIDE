// SymbioteIDE - User Experience Modes
// Easy/Interactive/Manual Modes System

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserMode {
    Easy,
    Interactive,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeManager {
    pub current_mode: UserMode,
}

impl ModeManager {
    pub fn new() -> Self {
        Self {
            current_mode: UserMode::Interactive,
        }
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}
