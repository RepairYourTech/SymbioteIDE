//! # Codebase Index Engine Implementation
//! 
//! Core implementation of the codebase indexing engine.

use super::*;

impl WorkspaceIndex {
    /// Create new workspace index
    pub fn new(workspace_id: String, workspace_path: String) -> Self {
        Self {
            workspace_id,
            workspace_path,
            files: HashMap::new(),
            symbols: HashMap::new(),
            dependencies: HashMap::new(),
            language_stats: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    /// Update file in index
    pub async fn update_file(&mut self, file_path: &str, analysis: FileAnalysis) -> Result<()> {
        // Update file analysis
        self.files.insert(file_path.to_string(), analysis.clone());
        
        // Update symbols
        for symbol in &analysis.symbols {
            self.symbols.entry(symbol.name.clone())
                .or_insert_with(Vec::new)
                .push(symbol.clone());
        }
        
        // Update dependencies
        self.dependencies.insert(file_path.to_string(), analysis.dependencies.clone());
        
        // Update language stats
        let lang_stats = self.language_stats.entry(analysis.language.clone())
            .or_insert_with(|| LanguageStats {
                file_count: 0,
                line_count: 0,
                symbol_count: 0,
                complexity_score: 0.0,
            });
        
        lang_stats.file_count += 1;
        lang_stats.line_count += analysis.line_count as u64;
        lang_stats.symbol_count += analysis.symbols.len();
        lang_stats.complexity_score += analysis.complexity_score;
        
        self.last_updated = Utc::now();
        Ok(())
    }

    /// Remove file from index
    pub async fn remove_file(&mut self, file_path: &str) -> Result<()> {
        if let Some(analysis) = self.files.remove(file_path) {
            // Remove symbols
            for symbol in &analysis.symbols {
                if let Some(symbol_list) = self.symbols.get_mut(&symbol.name) {
                    symbol_list.retain(|s| s.file_path != file_path);
                    if symbol_list.is_empty() {
                        self.symbols.remove(&symbol.name);
                    }
                }
            }
            
            // Remove dependencies
            self.dependencies.remove(file_path);
            
            // Update language stats
            if let Some(lang_stats) = self.language_stats.get_mut(&analysis.language) {
                lang_stats.file_count = lang_stats.file_count.saturating_sub(1);
                lang_stats.line_count = lang_stats.line_count.saturating_sub(analysis.line_count as u64);
                lang_stats.symbol_count = lang_stats.symbol_count.saturating_sub(analysis.symbols.len());
                lang_stats.complexity_score -= analysis.complexity_score;
                
                if lang_stats.file_count == 0 {
                    self.language_stats.remove(&analysis.language);
                }
            }
        }
        
        self.last_updated = Utc::now();
        Ok(())
    }

    /// Get workspace path
    pub fn get_path(&self) -> String {
        self.workspace_path.clone()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> WorkspaceIndexStats {
        let total_files = self.files.len();
        let total_lines = self.files.values().map(|f| f.line_count as u64).sum();
        let total_symbols = self.symbols.values().map(|v| v.len()).sum();
        
        WorkspaceIndexStats {
            total_files,
            total_lines,
            total_symbols,
            languages: self.language_stats.clone(),
            index_size_bytes: self.calculate_index_size(),
            last_updated: self.last_updated,
        }
    }

    /// Get file dependencies
    pub async fn get_file_dependencies(&self, file_path: &str) -> Result<Vec<FileDependency>> {
        Ok(self.dependencies.get(file_path).cloned().unwrap_or_default())
    }

    /// Get symbol references
    pub async fn get_symbol_references(&self, symbol: &str) -> Result<Vec<SymbolReference>> {
        // Would implement symbol reference finding
        Ok(Vec::new())
    }

    fn calculate_index_size(&self) -> u64 {
        // Rough calculation of index size
        self.files.len() as u64 * 1024 + self.symbols.len() as u64 * 512
    }
}

impl GlobalCodebaseIndex {
    /// Create new global index
    pub fn new() -> Self {
        Self {
            workspace_indexes: HashMap::new(),
            global_symbols: HashMap::new(),
            cross_workspace_dependencies: HashMap::new(),
            global_stats: GlobalIndexStats {
                total_workspaces: 0,
                total_files: 0,
                total_lines: 0,
                total_symbols: 0,
                index_size_bytes: 0,
                last_updated: Utc::now(),
            },
            last_updated: Utc::now(),
        }
    }

    /// Add workspace index
    pub async fn add_workspace_index(&mut self, workspace_id: &str, workspace_index: &WorkspaceIndex) -> Result<()> {
        self.workspace_indexes.insert(workspace_id.to_string(), workspace_index.workspace_path.clone());
        
        // Update global symbols
        for (symbol_name, symbols) in &workspace_index.symbols {
            let global_refs = self.global_symbols.entry(symbol_name.clone())
                .or_insert_with(Vec::new);
            
            for symbol in symbols {
                global_refs.push(GlobalSymbolReference {
                    workspace_id: workspace_id.to_string(),
                    symbol: symbol.clone(),
                });
            }
        }
        
        self.update_global_stats().await;
        Ok(())
    }

    /// Update file in global index
    pub async fn update_file(&mut self, workspace_id: &str, file_path: &str, analysis: &FileAnalysis) -> Result<()> {
        // Update global symbols for this file
        for symbol in &analysis.symbols {
            let global_refs = self.global_symbols.entry(symbol.name.clone())
                .or_insert_with(Vec::new);
            
            // Remove old references from this file
            global_refs.retain(|r| !(r.workspace_id == workspace_id && r.symbol.file_path == file_path));
            
            // Add new reference
            global_refs.push(GlobalSymbolReference {
                workspace_id: workspace_id.to_string(),
                symbol: symbol.clone(),
            });
        }
        
        self.update_global_stats().await;
        Ok(())
    }

    /// Remove file from global index
    pub async fn remove_file(&mut self, workspace_id: &str, file_path: &str) -> Result<()> {
        // Remove symbols from this file
        for symbol_refs in self.global_symbols.values_mut() {
            symbol_refs.retain(|r| !(r.workspace_id == workspace_id && r.symbol.file_path == file_path));
        }
        
        // Remove empty symbol entries
        self.global_symbols.retain(|_, refs| !refs.is_empty());
        
        self.update_global_stats().await;
        Ok(())
    }

    /// Get global statistics
    pub async fn get_stats(&self) -> GlobalIndexStats {
        self.global_stats.clone()
    }

    async fn update_global_stats(&mut self) {
        self.global_stats.total_workspaces = self.workspace_indexes.len();
        self.global_stats.total_symbols = self.global_symbols.values().map(|v| v.len()).sum();
        self.global_stats.last_updated = Utc::now();
        self.last_updated = Utc::now();
    }
}

/// Global symbol reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSymbolReference {
    pub workspace_id: String,
    pub symbol: SymbolDefinition,
}

impl Default for GlobalCodebaseIndex {
    fn default() -> Self {
        Self::new()
    }
}
