// SymbioteIDE - Visual Programming Canvas
// Visual Programming System

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualCanvas {
    pub name: String,
}

impl VisualCanvas {
    pub fn new() -> Self {
        Self {
            name: "Visual Canvas".to_string(),
        }
    }
}

impl Default for VisualCanvas {
    fn default() -> Self {
        Self::new()
    }
}
