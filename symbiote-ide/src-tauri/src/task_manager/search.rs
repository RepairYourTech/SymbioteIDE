use crate::task_manager::core::*;
use crate::codebase_intelligence::CodebaseIntelligence;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Task search and filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilter {
    pub project_ids: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
    pub statuses: Option<Vec<TaskStatus>>,
    pub priorities: Option<Vec<TaskPriority>>,
    pub task_types: Option<Vec<TaskType>>,
    pub tags: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
    pub due_date_range: Option<(u64, u64)>,
    pub created_date_range: Option<(u64, u64)>,
    pub text_search: Option<String>,
    pub has_blockers: Option<bool>,
    pub overdue: Option<bool>,
    pub components: Option<Vec<String>>,
    pub files_affected: Option<Vec<String>>,
}

/// Task search result with relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSearchResult {
    pub task: Task,
    pub relevance_score: f32,
    pub match_reasons: Vec<String>,
}

/// Advanced search capabilities for tasks
pub struct TaskSearchEngine {
    codebase_intelligence: Option<Arc<CodebaseIntelligence>>,
}

impl TaskSearchEngine {
    pub fn new(codebase_intelligence: Option<Arc<CodebaseIntelligence>>) -> Self {
        Self {
            codebase_intelligence,
        }
    }
    
    /// Semantic search using Qdrant (when available)
    pub async fn semantic_search(
        &self,
        query: &str,
        tasks: &HashMap<String, Task>,
        limit: usize,
        filter: Option<TaskFilter>,
    ) -> Result<Vec<TaskSearchResult>, TaskManagerError> {
        match &self.codebase_intelligence {
            Some(intelligence) => {
                // Use Qdrant for semantic search
                let search_results = intelligence
                    .semantic_search_tasks(query, limit)
                    .await
                    .map_err(|e| TaskManagerError::SearchError(e.to_string()))?;
                
                Ok(search_results)
            },
            None => {
                // Fallback to text-based search
                self.text_search(query, tasks, limit, filter).await
            }
        }
    }
    
    /// Text-based search (SQLite fallback)
    pub async fn text_search(
        &self,
        query: &str,
        tasks: &HashMap<String, Task>,
        limit: usize,
        filter: Option<TaskFilter>,
    ) -> Result<Vec<TaskSearchResult>, TaskManagerError> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        for task in tasks.values() {
            // Apply filter first
            if let Some(ref filter) = filter {
                if !self.matches_filter(task, filter) {
                    continue;
                }
            }
            
            let mut relevance_score = 0.0;
            let mut match_reasons = Vec::new();
            
            // Title match (highest weight)
            if task.title.to_lowercase().contains(&query_lower) {
                relevance_score += 10.0;
                match_reasons.push("Title match".to_string());
            }
            
            // Description match
            if let Some(ref desc) = task.description {
                if desc.to_lowercase().contains(&query_lower) {
                    relevance_score += 5.0;
                    match_reasons.push("Description match".to_string());
                }
            }
            
            // Tags match
            for tag in &task.tags {
                if tag.to_lowercase().contains(&query_lower) {
                    relevance_score += 3.0;
                    match_reasons.push(format!("Tag match: {}", tag));
                }
            }
            
            // Components match
            for component in &task.components {
                if component.to_lowercase().contains(&query_lower) {
                    relevance_score += 2.0;
                    match_reasons.push(format!("Component match: {}", component));
                }
            }
            
            // Files affected match
            for file in &task.files_affected {
                if file.to_lowercase().contains(&query_lower) {
                    relevance_score += 2.0;
                    match_reasons.push(format!("File match: {}", file));
                }
            }
            
            // Metadata match
            for (key, value) in &task.metadata {
                if key.to_lowercase().contains(&query_lower) || value.to_lowercase().contains(&query_lower) {
                    relevance_score += 1.0;
                    match_reasons.push(format!("Metadata match: {} = {}", key, value));
                }
            }
            
            if relevance_score > 0.0 {
                results.push(TaskSearchResult {
                    task: task.clone(),
                    relevance_score,
                    match_reasons,
                });
            }
        }
        
        // Sort by relevance score (descending)
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        // Limit results
        results.truncate(limit);
        
        Ok(results)
    }
    
    /// Find related tasks using knowledge graph (when available)
    pub async fn find_related_tasks(
        &self,
        task_id: &str,
        tasks: &HashMap<String, Task>,
        max_depth: u8,
    ) -> Result<Vec<Task>, TaskManagerError> {
        match &self.codebase_intelligence {
            Some(intelligence) => {
                let related_task_ids = intelligence
                    .find_related_task_nodes(task_id, max_depth)
                    .await
                    .map_err(|e| TaskManagerError::SearchError(e.to_string()))?;
                
                let related_tasks = related_task_ids
                    .into_iter()
                    .filter_map(|id| tasks.get(&id).cloned())
                    .collect();
                
                Ok(related_tasks)
            },
            None => {
                // Fallback to basic relationship analysis
                self.find_basic_related_tasks(task_id, tasks).await
            }
        }
    }
    
    /// Basic related task finding (fallback)
    async fn find_basic_related_tasks(
        &self,
        task_id: &str,
        tasks: &HashMap<String, Task>,
    ) -> Result<Vec<Task>, TaskManagerError> {
        let target_task = tasks.get(task_id)
            .ok_or_else(|| TaskManagerError::TaskNotFound(task_id.to_string()))?;
        
        let mut related_tasks = Vec::new();
        
        for task in tasks.values() {
            if task.id == task_id {
                continue;
            }
            
            let mut is_related = false;
            
            // Same project
            if task.project_id == target_task.project_id && target_task.project_id.is_some() {
                is_related = true;
            }
            
            // Same epic
            if task.epic_id == target_task.epic_id && target_task.epic_id.is_some() {
                is_related = true;
            }
            
            // Parent-child relationship
            if task.parent_id == Some(target_task.id.clone()) || target_task.parent_id == Some(task.id.clone()) {
                is_related = true;
            }
            
            // Shared files
            let shared_files: Vec<_> = task.files_affected.iter()
                .filter(|file| target_task.files_affected.contains(file))
                .collect();
            if !shared_files.is_empty() {
                is_related = true;
            }
            
            // Shared components
            let shared_components: Vec<_> = task.components.iter()
                .filter(|component| target_task.components.contains(component))
                .collect();
            if !shared_components.is_empty() {
                is_related = true;
            }
            
            // Shared tags
            let shared_tags: Vec<_> = task.tags.iter()
                .filter(|tag| target_task.tags.contains(tag))
                .collect();
            if shared_tags.len() >= 2 {
                is_related = true;
            }
            
            if is_related {
                related_tasks.push(task.clone());
            }
        }
        
        Ok(related_tasks)
    }
    
    /// Apply task filter
    pub fn matches_filter(&self, task: &Task, filter: &TaskFilter) -> bool {
        // Project filter
        if let Some(ref project_ids) = filter.project_ids {
            if let Some(ref task_project_id) = task.project_id {
                if !project_ids.contains(task_project_id) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Assignee filter
        if let Some(ref assignees) = filter.assignees {
            if let Some(ref task_assignee) = task.assigned_to {
                if !assignees.contains(task_assignee) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Status filter
        if let Some(ref statuses) = filter.statuses {
            if !statuses.contains(&task.status) {
                return false;
            }
        }
        
        // Priority filter
        if let Some(ref priorities) = filter.priorities {
            if !priorities.contains(&task.priority) {
                return false;
            }
        }
        
        // Task type filter
        if let Some(ref task_types) = filter.task_types {
            if !task_types.contains(&task.task_type) {
                return false;
            }
        }
        
        // Tags filter
        if let Some(ref tags) = filter.tags {
            if !tags.iter().any(|tag| task.tags.contains(tag)) {
                return false;
            }
        }
        
        // Labels filter
        if let Some(ref labels) = filter.labels {
            if !labels.iter().any(|label| task.labels.contains(label)) {
                return false;
            }
        }
        
        // Due date range filter
        if let Some((start, end)) = filter.due_date_range {
            if let Some(due_date) = task.due_date {
                if due_date < start || due_date > end {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Created date range filter
        if let Some((start, end)) = filter.created_date_range {
            if task.created_at < start || task.created_at > end {
                return false;
            }
        }
        
        // Blockers filter
        if let Some(has_blockers) = filter.has_blockers {
            if has_blockers && task.blockers.is_empty() {
                return false;
            }
            if !has_blockers && !task.blockers.is_empty() {
                return false;
            }
        }
        
        // Overdue filter
        if let Some(overdue) = filter.overdue {
            let is_overdue = task.due_date.map_or(false, |due| {
                due < current_timestamp() && task.status != TaskStatus::Done
            });
            if overdue != is_overdue {
                return false;
            }
        }
        
        // Components filter
        if let Some(ref components) = filter.components {
            if !components.iter().any(|component| task.components.contains(component)) {
                return false;
            }
        }
        
        // Files affected filter
        if let Some(ref files) = filter.files_affected {
            if !files.iter().any(|file| task.files_affected.contains(file)) {
                return false;
            }
        }
        
        true
    }
    
    /// Get task suggestions based on context
    pub async fn suggest_tasks(
        &self,
        context: &str,
        tasks: &HashMap<String, Task>,
        limit: usize,
    ) -> Result<Vec<TaskSearchResult>, TaskManagerError> {
        // Extract keywords from context
        let keywords = self.extract_keywords(context);
        
        let mut suggestions = Vec::new();
        
        for task in tasks.values() {
            let mut relevance_score = 0.0;
            let mut match_reasons = Vec::new();
            
            // Check if task is incomplete
            if task.status == TaskStatus::Done {
                continue;
            }
            
            // Keyword matching
            for keyword in &keywords {
                if task.title.to_lowercase().contains(&keyword.to_lowercase()) {
                    relevance_score += 5.0;
                    match_reasons.push(format!("Title contains: {}", keyword));
                }
                
                if let Some(ref desc) = task.description {
                    if desc.to_lowercase().contains(&keyword.to_lowercase()) {
                        relevance_score += 3.0;
                        match_reasons.push(format!("Description contains: {}", keyword));
                    }
                }
                
                if task.tags.iter().any(|tag| tag.to_lowercase().contains(&keyword.to_lowercase())) {
                    relevance_score += 2.0;
                    match_reasons.push(format!("Tag contains: {}", keyword));
                }
            }
            
            // Priority boost for high priority tasks
            match task.priority {
                TaskPriority::Critical => relevance_score += 3.0,
                TaskPriority::High => relevance_score += 2.0,
                TaskPriority::Medium => relevance_score += 1.0,
                TaskPriority::Normal => relevance_score += 1.0,
                TaskPriority::Low => {},
            }
            
            // Boost for overdue tasks
            if let Some(due_date) = task.due_date {
                if due_date < current_timestamp() {
                    relevance_score += 4.0;
                    match_reasons.push("Overdue task".to_string());
                }
            }
            
            if relevance_score > 0.0 {
                suggestions.push(TaskSearchResult {
                    task: task.clone(),
                    relevance_score,
                    match_reasons,
                });
            }
        }
        
        // Sort by relevance
        suggestions.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        suggestions.truncate(limit);
        
        Ok(suggestions)
    }
    
    /// Extract keywords from context
    fn extract_keywords(&self, context: &str) -> Vec<String> {
        let stop_words = vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "is", "are", "was", "were"
        ];
        
        context
            .split_whitespace()
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
            .filter(|word| word.len() > 2 && !stop_words.contains(&word.as_str()))
            .collect()
    }
}
