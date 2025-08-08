use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;

use crate::codebase_intelligence::{CodebaseIntelligence, ContextQuery, QueryType, QueryScope, AgentContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub id: String,
    pub scope: String,
    pub key: String,
    pub data: String,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextScope {
    pub name: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub entries: HashMap<String, ContextEntry>,
}

pub struct ContextManager {
    scopes: HashMap<String, ContextScope>,
    global_context: HashMap<String, String>,
    codebase_intelligence: Option<Arc<CodebaseIntelligence>>,
}

impl ContextManager {
    pub fn new() -> Self {
        let mut manager = Self {
            scopes: HashMap::new(),
            global_context: HashMap::new(),
            codebase_intelligence: None,
        };

        // Initialize default scopes
        manager.create_scope("global", None);
        manager.create_scope("workspace", Some("global"));
        manager.create_scope("project", Some("workspace"));
        manager.create_scope("file", Some("project"));
        manager.create_scope("agent", Some("project"));

        manager
    }

    pub fn set_codebase_intelligence(&mut self, intelligence: Arc<CodebaseIntelligence>) {
        self.codebase_intelligence = Some(intelligence);
    }

    pub async fn get_agent_context(&self, agent_id: &str, task_context: &str, max_tokens: usize) -> Result<AgentContext> {
        if let Some(intelligence) = &self.codebase_intelligence {
            intelligence.get_agent_context(agent_id, task_context, max_tokens).await
        } else {
            // Fallback to basic context if intelligence not available
            let basic_context = self.get_context("agent", &vec![])?;
            Ok(AgentContext {
                entities: vec![],
                relationships: vec![],
                metadata: crate::codebase_intelligence::ContextMetadata {
                    sources: vec!["basic".to_string()],
                    retrieval_time_ms: 0,
                    fusion_strategy_used: "fallback".to_string(),
                    total_candidates: 0,
                    filtered_candidates: 0,
                },
                confidence: 0.5,
            })
        }
    }

    pub async fn query_codebase(&self, query_text: &str, max_results: usize) -> Result<AgentContext> {
        if let Some(intelligence) = &self.codebase_intelligence {
            let query = ContextQuery {
                query_text: query_text.to_string(),
                query_type: QueryType::Hybrid,
                scope: QueryScope::Global,
                filters: vec![],
                max_results,
                include_relationships: true,
                fusion_strategy: crate::codebase_intelligence::FusionStrategy::HybridFusion,
            };
            
            let context_result = intelligence.query_context(query).await?;
            Ok(context_result)
        } else {
            // Fallback
            Ok(AgentContext {
                entities: vec![],
                relationships: vec![],
                metadata: crate::codebase_intelligence::ContextMetadata {
                    sources: vec!["fallback".to_string()],
                    retrieval_time_ms: 0,
                    fusion_strategy_used: "fallback".to_string(),
                    total_candidates: 0,
                    filtered_candidates: 0,
                },
                confidence: 0.0,
            })
        }
    }

    pub fn create_scope(&mut self, name: &str, parent: Option<&str>) -> Result<()> {
        let scope = ContextScope {
            name: name.to_string(),
            parent: parent.map(|p| p.to_string()),
            children: Vec::new(),
            entries: HashMap::new(),
        };

        // Add to parent's children if parent exists
        if let Some(parent_name) = parent {
            if let Some(parent_scope) = self.scopes.get_mut(parent_name) {
                parent_scope.children.push(name.to_string());
            }
        }

        self.scopes.insert(name.to_string(), scope);
        Ok(())
    }

    pub fn get_context(&self, scope: &str, filters: &[String]) -> Result<String> {
        let mut context_data = Vec::new();

        // Get context from the specified scope and all parent scopes
        self.collect_context_recursive(scope, filters, &mut context_data)?;

        // Format context data
        let formatted_context = self.format_context_data(&context_data);
        Ok(formatted_context)
    }

    fn collect_context_recursive<'a>(&'a self, scope_name: &str, filters: &[String], context_data: &mut Vec<&'a ContextEntry>) -> Result<()> {
        let scope = self.scopes.get(scope_name)
            .ok_or_else(|| anyhow!("Scope not found: {}", scope_name))?;

        // Collect entries from current scope
        for entry in scope.entries.values() {
            if filters.is_empty() || filters.iter().any(|filter| {
                entry.key.contains(filter) || 
                entry.metadata.values().any(|v| v.contains(filter))
            }) {
                context_data.push(entry);
            }
        }

        // Recursively collect from parent scope
        if let Some(parent_name) = &scope.parent {
            self.collect_context_recursive(parent_name, filters, context_data)?;
        }

        Ok(())
    }

    fn format_context_data(&self, context_data: &[&ContextEntry]) -> String {
        let mut formatted = String::new();
        
        // Group by scope
        let mut by_scope: HashMap<String, Vec<&ContextEntry>> = HashMap::new();
        for entry in context_data {
            by_scope.entry(entry.scope.clone()).or_insert_with(Vec::new).push(entry);
        }

        for (scope, entries) in by_scope {
            formatted.push_str(&format!("=== {} CONTEXT ===\n", scope.to_uppercase()));
            
            for entry in entries {
                formatted.push_str(&format!("{}:\n{}\n\n", entry.key, entry.data));
            }
        }

        formatted
    }

    pub fn update_context(&mut self, scope: &str, data: &str) -> Result<()> {
        // Parse the data to extract key-value pairs
        let entries = self.parse_context_data(data)?;

        let scope_obj = self.scopes.get_mut(scope)
            .ok_or_else(|| anyhow!("Scope not found: {}", scope))?;

        for (key, value) in entries {
            let entry_id = format!("{}_{}", scope, key);
            let now = Utc::now();

            if let Some(existing_entry) = scope_obj.entries.get_mut(&key) {
                // Update existing entry
                existing_entry.data = value;
                existing_entry.updated_at = now;
                existing_entry.access_count += 1;
            } else {
                // Create new entry
                let entry = ContextEntry {
                    id: entry_id,
                    scope: scope.to_string(),
                    key: key.clone(),
                    data: value,
                    metadata: HashMap::new(),
                    created_at: now,
                    updated_at: now,
                    access_count: 1,
                };
                scope_obj.entries.insert(key, entry);
            }
        }

        Ok(())
    }

    fn parse_context_data(&self, data: &str) -> Result<HashMap<String, String>> {
        let mut entries = HashMap::new();

        // Simple parsing - look for key: value patterns
        for line in data.lines() {
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                if !key.is_empty() && !value.is_empty() {
                    entries.insert(key, value);
                }
            }
        }

        // If no key-value pairs found, treat entire data as a single entry
        if entries.is_empty() {
            entries.insert("content".to_string(), data.to_string());
        }

        Ok(entries)
    }

    pub fn add_file_context(&mut self, file_path: &str, content: &str, language: &str) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "file".to_string());
        metadata.insert("language".to_string(), language.to_string());
        metadata.insert("path".to_string(), file_path.to_string());

        let entry = ContextEntry {
            id: format!("file_{}", file_path.replace(['/', '\\'], "_")),
            scope: "file".to_string(),
            key: file_path.to_string(),
            data: content.to_string(),
            metadata,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            access_count: 0,
        };

        if let Some(scope) = self.scopes.get_mut("file") {
            scope.entries.insert(file_path.to_string(), entry);
        }

        Ok(())
    }

    pub fn add_agent_context(&mut self, agent_id: &str, context_type: &str, data: &str) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), context_type.to_string());
        metadata.insert("agent_id".to_string(), agent_id.to_string());

        let entry = ContextEntry {
            id: format!("agent_{}_{}", agent_id, context_type),
            scope: "agent".to_string(),
            key: format!("{}_{}", agent_id, context_type),
            data: data.to_string(),
            metadata,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            access_count: 0,
        };

        if let Some(scope) = self.scopes.get_mut("agent") {
            scope.entries.insert(entry.key.clone(), entry);
        }

        Ok(())
    }

    pub fn get_context_for_agent(&self, agent_id: &str, context_types: &[String]) -> Result<String> {
        let mut filters = vec![agent_id.to_string()];
        filters.extend_from_slice(context_types);
        
        self.get_context("agent", &filters)
    }
}

// Global context manager instance
static mut CONTEXT_MANAGER: Option<ContextManager> = None;

pub fn get_context(scope: &str, filters: &[String]) -> Result<String> {
    unsafe {
        if CONTEXT_MANAGER.is_none() {
            CONTEXT_MANAGER = Some(ContextManager::new());
        }

        let manager = CONTEXT_MANAGER.as_ref().unwrap();
        manager.get_context(scope, filters)
    }
}

pub fn update_context(scope: &str, data: &str) -> Result<()> {
    unsafe {
        if CONTEXT_MANAGER.is_none() {
            CONTEXT_MANAGER = Some(ContextManager::new());
        }

        let manager = CONTEXT_MANAGER.as_mut().unwrap();
        manager.update_context(scope, data)
    }
}
