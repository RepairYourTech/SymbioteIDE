//! # Codebase Index Engine
//! 
//! Real-time codebase indexing and intelligence system that provides semantic
//! code understanding, symbol resolution, and AI-powered code insights.

pub mod engine;
pub mod indexer;
pub mod search;
pub mod symbols;
pub mod intelligence;
pub mod graph_strategies;

// Re-export main types
pub use engine::*;
pub use indexer::*;
pub use search::*;
pub use symbols::*;
pub use intelligence::*;
pub use graph_strategies::*;

use crate::{Result, SymbioteError};
use crate::context::ContextBus;
use crate::workspace::WorkspaceManager;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main codebase index engine providing real-time code intelligence
#[derive(Debug)]
pub struct CodebaseIndexEngine {
    /// Real-time file indexer
    indexer: CodebaseIndexer,
    
    /// Semantic search engine
    search_engine: CodebaseSearchEngine,
    
    /// Symbol resolution system
    symbol_resolver: SymbolResolver,
    
    /// AI-powered code intelligence
    code_intelligence: CodeIntelligence,

    /// Advanced graph strategies for codebase analysis
    graph_strategies: CodebaseGraphStrategies,

    /// Workspace-specific indexes
    workspace_indexes: Arc<RwLock<HashMap<String, WorkspaceIndex>>>,

    /// Global cross-workspace index
    global_index: Arc<RwLock<GlobalCodebaseIndex>>,
    
    /// Integration with context system
    context_bus: Option<Arc<ContextBus>>,
}

impl CodebaseIndexEngine {
    /// Create new codebase index engine
    pub fn new() -> Self {
        Self {
            indexer: CodebaseIndexer::new(),
            search_engine: CodebaseSearchEngine::new(),
            symbol_resolver: SymbolResolver::new(),
            code_intelligence: CodeIntelligence::new(),
            graph_strategies: CodebaseGraphStrategies::new(),
            workspace_indexes: Arc::new(RwLock::new(HashMap::new())),
            global_index: Arc::new(RwLock::new(GlobalCodebaseIndex::new())),
            context_bus: None,
        }
    }

    /// Create with context bus integration
    pub fn with_context_bus(context_bus: Arc<ContextBus>) -> Self {
        let mut engine = Self::new();
        engine.context_bus = Some(context_bus);
        engine
    }

    /// Index a workspace
    pub async fn index_workspace(&self, workspace_id: &str, workspace_path: &str) -> Result<()> {
        // Create workspace index
        let workspace_index = self.indexer.index_workspace(workspace_path).await?;
        
        // Store workspace index
        {
            let mut indexes = self.workspace_indexes.write().await;
            indexes.insert(workspace_id.to_string(), workspace_index.clone());
        }
        
        // Update global index
        {
            let mut global = self.global_index.write().await;
            global.add_workspace_index(workspace_id, &workspace_index).await?;
        }
        
        // Notify context bus
        if let Some(context_bus) = &self.context_bus {
            // Would integrate with context bus
        }
        
        Ok(())
    }

    /// Update file in index
    pub async fn update_file(&self, workspace_id: &str, file_path: &str, content: &str) -> Result<()> {
        // Parse and analyze file
        let file_analysis = self.indexer.analyze_file(file_path, content).await?;
        
        // Update workspace index
        {
            let mut indexes = self.workspace_indexes.write().await;
            if let Some(workspace_index) = indexes.get_mut(workspace_id) {
                workspace_index.update_file(file_path, file_analysis.clone()).await?;
            }
        }
        
        // Update global index
        {
            let mut global = self.global_index.write().await;
            global.update_file(workspace_id, file_path, &file_analysis).await?;
        }
        
        Ok(())
    }

    /// Remove file from index
    pub async fn remove_file(&self, workspace_id: &str, file_path: &str) -> Result<()> {
        // Remove from workspace index
        {
            let mut indexes = self.workspace_indexes.write().await;
            if let Some(workspace_index) = indexes.get_mut(workspace_id) {
                workspace_index.remove_file(file_path).await?;
            }
        }
        
        // Remove from global index
        {
            let mut global = self.global_index.write().await;
            global.remove_file(workspace_id, file_path).await?;
        }
        
        Ok(())
    }

    /// Search code across workspace
    pub async fn search_workspace(&self, workspace_id: &str, query: &CodeSearchQuery) -> Result<Vec<CodeSearchResult>> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            self.search_engine.search_workspace(workspace_index, query).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Search code across all workspaces
    pub async fn search_global(&self, query: &CodeSearchQuery) -> Result<Vec<CodeSearchResult>> {
        let global = self.global_index.read().await;
        self.search_engine.search_global(&global, query).await
    }

    /// Resolve symbol in workspace
    pub async fn resolve_symbol(&self, workspace_id: &str, symbol: &str, file_path: &str) -> Result<Vec<SymbolDefinition>> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            self.symbol_resolver.resolve_symbol(workspace_index, symbol, file_path).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get code intelligence for file
    pub async fn get_code_intelligence(&self, workspace_id: &str, file_path: &str) -> Result<CodeIntelligenceResult> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            self.code_intelligence.analyze_file(workspace_index, file_path).await
        } else {
            Err(SymbioteError::NotFound("Workspace index not found".to_string()))
        }
    }

    /// Get workspace statistics
    pub async fn get_workspace_stats(&self, workspace_id: &str) -> Result<WorkspaceIndexStats> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            Ok(workspace_index.get_stats().await)
        } else {
            Err(SymbioteError::NotFound("Workspace index not found".to_string()))
        }
    }

    /// Get global statistics
    pub async fn get_global_stats(&self) -> GlobalIndexStats {
        let global = self.global_index.read().await;
        global.get_stats().await
    }

    /// Refresh workspace index
    pub async fn refresh_workspace(&self, workspace_id: &str) -> Result<()> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            let workspace_path = workspace_index.get_path();
            drop(indexes); // Release read lock
            
            // Re-index workspace
            self.index_workspace(workspace_id, &workspace_path).await
        } else {
            Err(SymbioteError::NotFound("Workspace index not found".to_string()))
        }
    }

    /// Get file dependencies
    pub async fn get_file_dependencies(&self, workspace_id: &str, file_path: &str) -> Result<Vec<FileDependency>> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            workspace_index.get_file_dependencies(file_path).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get symbol references
    pub async fn get_symbol_references(&self, workspace_id: &str, symbol: &str) -> Result<Vec<SymbolReference>> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            workspace_index.get_symbol_references(symbol).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get code suggestions for AI agents
    pub async fn get_ai_suggestions(&self, workspace_id: &str, context: &AIContext) -> Result<Vec<AISuggestion>> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            self.code_intelligence.get_ai_suggestions(workspace_index, context).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Build comprehensive code graph using advanced strategies
    pub async fn build_comprehensive_graph(&self, workspace_id: &str) -> Result<ComprehensiveCodeGraph> {
        let indexes = self.workspace_indexes.read().await;
        if let Some(workspace_index) = indexes.get(workspace_id) {
            self.graph_strategies.build_comprehensive_graph(workspace_index).await
        } else {
            Err(SymbioteError::NotFound("Workspace index not found".to_string()))
        }
    }

    /// Analyze code vulnerabilities using graph strategies
    pub async fn analyze_vulnerabilities(&self, workspace_id: &str) -> Result<Vec<VulnerabilityReport>> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        self.graph_strategies.analyze_vulnerabilities(&graph).await
    }

    /// Extract semantic features for LLM understanding
    pub async fn extract_semantic_features(&self, workspace_id: &str) -> Result<SemanticFeatures> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        self.graph_strategies.extract_semantic_features(&graph).await
    }

    /// Build Code Property Graph (CPG) for workspace
    pub async fn build_code_property_graph(&self, workspace_id: &str) -> Result<CodePropertyGraph> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        if let Some(cpg) = graph.code_property_graph {
            Ok(cpg)
        } else {
            Err(SymbioteError::NotFound("CPG not available".to_string()))
        }
    }

    /// Get Abstract Syntax Trees for all files
    pub async fn get_abstract_syntax_trees(&self, workspace_id: &str) -> Result<HashMap<String, AbstractSyntaxTree>> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        Ok(graph.abstract_syntax_trees)
    }

    /// Build Control Flow Graph for workspace
    pub async fn build_control_flow_graph(&self, workspace_id: &str) -> Result<ControlFlowGraph> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        if let Some(cfg) = graph.control_flow_graph {
            Ok(cfg)
        } else {
            Err(SymbioteError::NotFound("CFG not available".to_string()))
        }
    }

    /// Build Data Flow Graph for workspace
    pub async fn build_data_flow_graph(&self, workspace_id: &str) -> Result<DataFlowGraph> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        if let Some(dfg) = graph.data_flow_graph {
            Ok(dfg)
        } else {
            Err(SymbioteError::NotFound("DFG not available".to_string()))
        }
    }

    /// Build Program Dependency Graph for workspace
    pub async fn build_program_dependency_graph(&self, workspace_id: &str) -> Result<ProgramDependencyGraph> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        if let Some(pdg) = graph.program_dependency_graph {
            Ok(pdg)
        } else {
            Err(SymbioteError::NotFound("PDG not available".to_string()))
        }
    }

    /// Get semantic code graph for LLM integration
    pub async fn get_semantic_code_graph(&self, workspace_id: &str) -> Result<SemanticCodeGraph> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        if let Some(scg) = graph.semantic_code_graph {
            Ok(scg)
        } else {
            Err(SymbioteError::NotFound("SCG not available".to_string()))
        }
    }

    /// Extract Graph Neural Network features
    pub async fn extract_gnn_features(&self, workspace_id: &str) -> Result<GNNFeatures> {
        let graph = self.build_comprehensive_graph(workspace_id).await?;
        if let Some(gnn) = graph.gnn_features {
            Ok(gnn)
        } else {
            Err(SymbioteError::NotFound("GNN features not available".to_string()))
        }
    }
}

/// Workspace-specific code index
#[derive(Debug, Clone)]
pub struct WorkspaceIndex {
    pub workspace_id: String,
    pub workspace_path: String,
    pub files: HashMap<String, FileAnalysis>,
    pub symbols: HashMap<String, Vec<SymbolDefinition>>,
    pub dependencies: HashMap<String, Vec<FileDependency>>,
    pub language_stats: HashMap<String, LanguageStats>,
    pub last_updated: DateTime<Utc>,
}

/// Global cross-workspace index
#[derive(Debug, Clone)]
pub struct GlobalCodebaseIndex {
    pub workspace_indexes: HashMap<String, String>, // workspace_id -> path
    pub global_symbols: HashMap<String, Vec<GlobalSymbolReference>>,
    pub cross_workspace_dependencies: HashMap<String, Vec<CrossWorkspaceDependency>>,
    pub global_stats: GlobalIndexStats,
    pub last_updated: DateTime<Utc>,
}

/// File analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub file_path: String,
    pub language: String,
    pub size_bytes: u64,
    pub line_count: u32,
    pub symbols: Vec<SymbolDefinition>,
    pub imports: Vec<ImportStatement>,
    pub exports: Vec<ExportStatement>,
    pub dependencies: Vec<FileDependency>,
    pub complexity_score: f64,
    pub last_modified: DateTime<Utc>,
}

/// Code search query
#[derive(Debug, Clone)]
pub struct CodeSearchQuery {
    pub query: String,
    pub search_type: SearchType,
    pub language_filter: Option<String>,
    pub file_pattern: Option<String>,
    pub symbol_type: Option<SymbolType>,
    pub limit: Option<usize>,
}

/// Search types
#[derive(Debug, Clone)]
pub enum SearchType {
    Text,
    Symbol,
    Semantic,
    Regex,
    Fuzzy,
}

/// Code search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    pub file_path: String,
    pub line_number: u32,
    pub column: u32,
    pub match_text: String,
    pub context_before: String,
    pub context_after: String,
    pub relevance_score: f64,
    pub symbol_info: Option<SymbolDefinition>,
}

/// Symbol definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDefinition {
    pub name: String,
    pub symbol_type: SymbolType,
    pub file_path: String,
    pub line_number: u32,
    pub column: u32,
    pub scope: String,
    pub visibility: Visibility,
    pub documentation: Option<String>,
    pub signature: Option<String>,
}

/// Symbol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Interface,
    Variable,
    Constant,
    Type,
    Module,
    Namespace,
    Enum,
    Struct,
}

/// Symbol visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
}

/// Workspace index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceIndexStats {
    pub total_files: usize,
    pub total_lines: u64,
    pub total_symbols: usize,
    pub languages: HashMap<String, LanguageStats>,
    pub index_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// Language statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageStats {
    pub file_count: usize,
    pub line_count: u64,
    pub symbol_count: usize,
    pub complexity_score: f64,
}

/// Global index statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalIndexStats {
    pub total_workspaces: usize,
    pub total_files: usize,
    pub total_lines: u64,
    pub total_symbols: usize,
    pub index_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

// Placeholder types (would be fully implemented)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDependency;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolReference;

// Removed duplicate struct definition - using the one from engine.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossWorkspaceDependency;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStatement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStatement;

// Removed duplicate struct definitions - using the ones from intelligence.rs

impl Default for CodebaseIndexEngine {
    fn default() -> Self {
        Self::new()
    }
}
