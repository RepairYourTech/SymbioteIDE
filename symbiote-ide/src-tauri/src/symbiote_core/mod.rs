// Symbiote Core - The Brain of SymbioteIDE
// Contains the fundamental AI systems// Core Foundation modules

pub mod mcp_server;
pub mod ai_marketplace;
pub mod codebase_intelligence;
pub mod agents;
pub mod tokenizer;
pub mod context;  // Context Bus - Event-driven system (1000+ events/sec)
pub mod memory;   // Symbiote Memory - Team Intelligence
pub mod hybrid_intelligence;

// Re-export key types for clean API
pub use mcp_server::*;
pub use ai_marketplace::*;
pub use codebase_intelligence::*;
pub use agents::*;
pub use context::*;
pub use tokenizer::*;
pub use memory::*;
pub use hybrid_intelligence::*;
