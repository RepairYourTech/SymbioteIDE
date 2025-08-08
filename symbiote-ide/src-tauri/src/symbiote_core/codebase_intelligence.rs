// SymbioteIDE - Codebase Intelligence Module
// Qdrant + Neo4j Intelligence System

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseIntelligence {
    pub name: String,
}

impl CodebaseIntelligence {
    pub fn new() -> Self {
        Self {
            name: "Codebase Intelligence".to_string(),
        }
    }
}

impl Default for CodebaseIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export types from main modules for compatibility
pub use crate::qdrant_store::QdrantStore;
pub use crate::neo4j_graph::Neo4jGraph;

// Stub parser types
#[derive(Debug, Clone)]
pub struct SymbioteParser;

#[derive(Debug, Clone)]
pub struct UnifiedAST;
