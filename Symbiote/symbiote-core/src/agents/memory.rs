//! # Memory System for Symbiotes
//! 
//! Advanced memory management for Symbiotes including short-term, long-term,
//! and working memory with intelligent compression and retrieval.

use super::*;
use crate::{Result, SymbioteError};
use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Symbiote memory system
#[derive(Debug, Clone)]
pub struct SymbioteMemory {
    /// Short-term memory for immediate context
    pub short_term: VecDeque<MemoryItem>,
    
    /// Long-term memory for persistent knowledge
    pub long_term: HashMap<String, MemoryItem>,
    
    /// Working memory for active tasks
    pub working_memory: HashMap<String, serde_json::Value>,
    
    /// Memory configuration
    config: MemoryConfig,
    
    /// Memory metrics
    metrics: MemoryMetrics,
    
    /// Memory index for fast retrieval
    index: MemoryIndex,
}

impl SymbioteMemory {
    pub fn new() -> Self {
        Self {
            short_term: VecDeque::new(),
            long_term: HashMap::new(),
            working_memory: HashMap::new(),
            config: MemoryConfig::default(),
            metrics: MemoryMetrics::new(),
            index: MemoryIndex::new(),
        }
    }

    /// Initialize memory with execution context
    pub async fn initialize_context(&mut self, context: &ExecutionContext) -> Result<()> {
        // Initialize working memory with context
        self.working_memory.insert(
            "workspace_path".to_string(),
            serde_json::Value::String(context.workspace_path.clone())
        );

        if let Some(ref git_worktree) = context.git_worktree {
            self.working_memory.insert(
                "git_worktree".to_string(),
                serde_json::Value::String(git_worktree.clone())
            );
        }

        // Store environment variables
        for (key, value) in &context.environment_variables {
            self.working_memory.insert(
                format!("env_{}", key),
                serde_json::Value::String(value.clone())
            );
        }

        tracing::debug!("Initialized memory with execution context");
        Ok(())
    }

    /// Record the start of a task
    pub async fn record_task_start(&mut self, task: &SymbioteTask) -> Result<()> {
        let memory_item = MemoryItem {
            id: format!("task_start_{}", task.id),
            content: format!("Started task: {:?} - {}", task.task_type, task.description),
            item_type: MemoryType::TaskEvent,
            importance: 0.7,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 1,
            tags: vec!["task".to_string(), "start".to_string()],
            metadata: HashMap::new(),
        };

        self.add_to_short_term(memory_item).await?;
        
        // Update working memory
        self.working_memory.insert(
            "current_task_id".to_string(),
            serde_json::Value::String(task.id.clone())
        );

        Ok(())
    }

    /// Record task completion
    pub async fn record_task_completion(
        &mut self,
        task: &SymbioteTask,
        result: &Result<SymbioteResult>,
        execution_time: u64,
    ) -> Result<()> {
        let success = result.is_ok();
        let content = if success {
            format!("Completed task: {} successfully in {}ms", task.id, execution_time)
        } else {
            format!("Failed task: {} after {}ms", task.id, execution_time)
        };

        let memory_item = MemoryItem {
            id: format!("task_completion_{}", task.id),
            content,
            item_type: MemoryType::TaskEvent,
            importance: if success { 0.8 } else { 0.9 }, // Failures are more important to remember
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 1,
            tags: vec![
                "task".to_string(),
                "completion".to_string(),
                if success { "success".to_string() } else { "failure".to_string() }
            ],
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("execution_time".to_string(), serde_json::Value::Number(execution_time.into()));
                metadata.insert("success".to_string(), serde_json::Value::Bool(success));
                metadata
            },
        };

        self.add_to_short_term(memory_item).await?;

        // Clear current task from working memory
        self.working_memory.remove("current_task_id");

        // Update metrics
        self.metrics.tasks_recorded += 1;
        if success {
            self.metrics.successful_tasks += 1;
        } else {
            self.metrics.failed_tasks += 1;
        }

        Ok(())
    }

    /// Store a piece of information in memory
    pub async fn store(&mut self, content: String, item_type: MemoryType, importance: f64) -> Result<String> {
        let memory_item = MemoryItem {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            item_type,
            importance,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_accessed: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            access_count: 1,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        let id = memory_item.id.clone();
        self.add_to_short_term(memory_item).await?;
        Ok(id)
    }

    /// Retrieve memories by query
    pub async fn retrieve(&mut self, query: &str, limit: Option<usize>) -> Result<Vec<MemoryItem>> {
        let limit = limit.unwrap_or(10);
        let mut results = Vec::new();
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        // Search in short-term memory
        for item in &mut self.short_term {
            if Self::matches_query_item(item, query) {
                item.last_accessed = current_time;
                item.access_count += 1;
                results.push(item.clone());
            }
        }

        // Search in long-term memory
        for item in self.long_term.values_mut() {
            if Self::matches_query_item(item, query) {
                item.last_accessed = current_time;
                item.access_count += 1;
                results.push(item.clone());
            }
        }

        // Sort by relevance (importance + recency + access frequency)
        results.sort_by(|a, b| {
            let score_a = self.calculate_relevance_score(a, query);
            let score_b = self.calculate_relevance_score(b, query);
            score_b.partial_cmp(&score_a).unwrap()
        });

        results.truncate(limit);
        Ok(results)
    }

    /// Get memory statistics
    pub async fn get_statistics(&self) -> MemoryStatistics {
        MemoryStatistics {
            short_term_count: self.short_term.len(),
            long_term_count: self.long_term.len(),
            working_memory_count: self.working_memory.len(),
            total_memory_items: self.short_term.len() + self.long_term.len(),
            memory_usage_bytes: self.estimate_memory_usage(),
            compression_ratio: self.calculate_compression_ratio(),
            access_patterns: self.analyze_access_patterns(),
        }
    }

    /// Compress old memories
    pub async fn compress_memories(&mut self) -> Result<()> {
        // Move important short-term memories to long-term
        let mut to_promote = Vec::new();
        
        for item in &self.short_term {
            if item.importance > self.config.long_term_threshold {
                to_promote.push(item.clone());
            }
        }

        for item in to_promote {
            self.long_term.insert(item.id.clone(), item);
        }

        // Remove old short-term memories
        let cutoff_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - self.config.short_term_retention_seconds;
        self.short_term.retain(|item| item.created_at > cutoff_time || item.importance > self.config.long_term_threshold);

        // Compress long-term memory if it's too large
        if self.long_term.len() > self.config.max_long_term_items {
            let mut items: Vec<_> = self.long_term.values().cloned().collect();
            items.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
            
            self.long_term.clear();
            for item in items.into_iter().take(self.config.max_long_term_items) {
                self.long_term.insert(item.id.clone(), item);
            }
        }

        tracing::debug!("Compressed memories: {} short-term, {} long-term", 
                       self.short_term.len(), self.long_term.len());
        Ok(())
    }

    // Private helper methods

    async fn add_to_short_term(&mut self, item: MemoryItem) -> Result<()> {
        // Add to index
        self.index.add_item(&item).await?;
        
        // Add to short-term memory
        self.short_term.push_back(item);
        
        // Limit short-term memory size
        while self.short_term.len() > self.config.max_short_term_items {
            if let Some(old_item) = self.short_term.pop_front() {
                self.index.remove_item(&old_item.id).await?;
            }
        }

        Ok(())
    }

    fn matches_query_item(item: &MemoryItem, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        
        // Check content
        if item.content.to_lowercase().contains(&query_lower) {
            return true;
        }
        
        // Check tags
        for tag in &item.tags {
            if tag.to_lowercase().contains(&query_lower) {
                return true;
            }
        }
        
        false
    }

    fn calculate_relevance_score(&self, item: &MemoryItem, query: &str) -> f64 {
        let mut score = item.importance;
        
        // Boost score for recent items
        let age_seconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - item.created_at;
        let recency_boost = 1.0 / (1.0 + age_seconds as f64 / 3600.0); // Decay over hours
        score += recency_boost * 0.3;
        
        // Boost score for frequently accessed items
        let access_boost = (item.access_count as f64).ln() / 10.0;
        score += access_boost * 0.2;
        
        // Boost score for exact matches
        if item.content.to_lowercase().contains(&query.to_lowercase()) {
            score += 0.5;
        }
        
        score
    }

    fn estimate_memory_usage(&self) -> usize {
        let mut usage = 0;
        
        // Estimate short-term memory usage
        for item in &self.short_term {
            usage += item.content.len() + 200; // Approximate overhead
        }
        
        // Estimate long-term memory usage
        for item in self.long_term.values() {
            usage += item.content.len() + 200; // Approximate overhead
        }
        
        // Estimate working memory usage
        for (key, value) in &self.working_memory {
            usage += key.len() + value.to_string().len() + 100;
        }
        
        usage
    }

    fn calculate_compression_ratio(&self) -> f64 {
        // Simple compression ratio calculation
        let total_items = self.short_term.len() + self.long_term.len();
        if total_items == 0 {
            return 1.0;
        }
        
        let compressed_items = self.long_term.len();
        compressed_items as f64 / total_items as f64
    }

    fn analyze_access_patterns(&self) -> Vec<AccessPattern> {
        let mut patterns = Vec::new();
        
        // Analyze memory type access patterns
        let mut type_access: HashMap<MemoryType, u64> = HashMap::new();
        
        for item in &self.short_term {
            *type_access.entry(item.item_type.clone()).or_insert(0) += item.access_count;
        }
        
        for item in self.long_term.values() {
            *type_access.entry(item.item_type.clone()).or_insert(0) += item.access_count;
        }
        
        for (memory_type, access_count) in type_access {
            patterns.push(AccessPattern {
                pattern_type: format!("{:?}", memory_type),
                access_frequency: access_count as f64,
                last_access: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            });
        }
        
        patterns
    }
}

/// Memory item stored in Symbiote memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub content: String,
    pub item_type: MemoryType,
    pub importance: f64,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u64,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of memory items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    TaskEvent,
    Knowledge,
    Experience,
    Pattern,
    Error,
    Success,
    Interaction,
    Context,
    Custom(String),
}

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_short_term_items: usize,
    pub max_long_term_items: usize,
    pub short_term_retention_seconds: u64,
    pub long_term_threshold: f64,
    pub compression_enabled: bool,
    pub indexing_enabled: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_short_term_items: 100,
            max_long_term_items: 1000,
            short_term_retention_seconds: 3600, // 1 hour
            long_term_threshold: 0.7,
            compression_enabled: true,
            indexing_enabled: true,
        }
    }
}

/// Memory metrics
#[derive(Debug, Clone)]
pub struct MemoryMetrics {
    pub tasks_recorded: u64,
    pub successful_tasks: u64,
    pub failed_tasks: u64,
    pub memories_created: u64,
    pub memories_accessed: u64,
    pub compressions_performed: u64,
    pub last_updated: u64,
}

impl MemoryMetrics {
    pub fn new() -> Self {
        Self {
            tasks_recorded: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            memories_created: 0,
            memories_accessed: 0,
            compressions_performed: 0,
            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatistics {
    pub short_term_count: usize,
    pub long_term_count: usize,
    pub working_memory_count: usize,
    pub total_memory_items: usize,
    pub memory_usage_bytes: usize,
    pub compression_ratio: f64,
    pub access_patterns: Vec<AccessPattern>,
}

/// Access pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub pattern_type: String,
    pub access_frequency: f64,
    pub last_access: u64,
}

/// Memory index for fast retrieval
#[derive(Debug, Clone)]
pub struct MemoryIndex {
    content_index: HashMap<String, Vec<String>>, // word -> memory IDs
    tag_index: HashMap<String, Vec<String>>,     // tag -> memory IDs
    type_index: HashMap<MemoryType, Vec<String>>, // type -> memory IDs
}

impl MemoryIndex {
    pub fn new() -> Self {
        Self {
            content_index: HashMap::new(),
            tag_index: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    pub async fn add_item(&mut self, item: &MemoryItem) -> Result<()> {
        // Index content words
        for word in item.content.split_whitespace() {
            let word_key = word.to_lowercase();
            self.content_index.entry(word_key)
                .or_insert_with(Vec::new)
                .push(item.id.clone());
        }

        // Index tags
        for tag in &item.tags {
            self.tag_index.entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(item.id.clone());
        }

        // Index by type
        self.type_index.entry(item.item_type.clone())
            .or_insert_with(Vec::new)
            .push(item.id.clone());

        Ok(())
    }

    pub async fn remove_item(&mut self, item_id: &str) -> Result<()> {
        // Remove from all indices
        for ids in self.content_index.values_mut() {
            ids.retain(|id| id != item_id);
        }

        for ids in self.tag_index.values_mut() {
            ids.retain(|id| id != item_id);
        }

        for ids in self.type_index.values_mut() {
            ids.retain(|id| id != item_id);
        }

        Ok(())
    }

    pub async fn search(&self, query: &str) -> Vec<String> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        // Search content index
        for word in query.split_whitespace() {
            let word_key = word.to_lowercase();
            if let Some(ids) = self.content_index.get(&word_key) {
                results.extend(ids.clone());
            }
        }

        // Search tag index
        if let Some(ids) = self.tag_index.get(&query_lower) {
            results.extend(ids.clone());
        }

        // Remove duplicates
        results.sort();
        results.dedup();
        results
    }
}
