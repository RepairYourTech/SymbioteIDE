use crate::task_manager::{
    core::*,
    database::TaskDatabase,
    events::{TaskEvent, TaskEventManager},
    search::{TaskSearchEngine, TaskFilter, TaskSearchResult},
    analytics::{TaskAnalytics, TaskAnalyticsEngine, ProjectAnalytics, TaskTrends},
    agent_coordination::{AgentCoordinationManager, AgentAssignmentRequest, CoordinationEvent, AgentActivitySummary},
};
use crate::codebase_intelligence::CodebaseIntelligence;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// Main task manager implementation
pub struct TaskManager {
    config: TaskManagerConfig,
    database: TaskDatabase,
    event_manager: TaskEventManager,
    search_engine: TaskSearchEngine,
    analytics_engine: TaskAnalyticsEngine,
    
    // In-memory caches for performance
    tasks: Arc<RwLock<HashMap<String, Task>>>,
    projects: Arc<RwLock<HashMap<String, Project>>>,
    epics: Arc<RwLock<HashMap<String, Epic>>>,
    dependencies: Arc<RwLock<HashMap<String, TaskDependency>>>,
    
    // Advanced features (optional)
    codebase_intelligence: Option<Arc<CodebaseIntelligence>>,
    
    // Agent coordination
    agent_coordinator: AgentCoordinationManager,
}

impl TaskManager {
    /// Create new task manager instance
    pub async fn new(
        pool: SqlitePool,
        config: TaskManagerConfig,
        codebase_intelligence: Option<Arc<CodebaseIntelligence>>,
    ) -> Result<Self, TaskManagerError> {
        let database = TaskDatabase::new(pool);
        
        // Initialize database schema
        database.initialize_schema().await?;
        
        let event_manager = TaskEventManager::new();
        let search_engine = TaskSearchEngine::new(codebase_intelligence.clone());
        let analytics_engine = TaskAnalyticsEngine::new();
        let agent_coordinator = AgentCoordinationManager::new();
        
        // Load existing data from database
        let tasks = Arc::new(RwLock::new(database.load_tasks().await?));
        let projects = Arc::new(RwLock::new(database.load_projects().await?));
        let epics = Arc::new(RwLock::new(database.load_epics().await?));
        let dependencies = Arc::new(RwLock::new(database.load_dependencies().await?));
        
        Ok(Self {
            config,
            database,
            event_manager,
            search_engine,
            analytics_engine,
            tasks,
            projects,
            epics,
            dependencies,
            codebase_intelligence,
            agent_coordinator,
        })
    }
    
    /// Create a new task
    pub async fn create_task(&self, mut task: Task) -> Result<String, TaskManagerError> {
        // Set creation timestamp
        task.created_at = current_timestamp();
        task.updated_at = task.created_at;
        
        // Generate ID if not provided
        if task.id.is_empty() {
            task.id = generate_id();
        }
        
        // Validate task
        self.validate_task(&task).await?;
        
        // Generate semantic embedding if advanced mode is enabled
        if self.config.enable_semantic_search {
            if let Some(ref intelligence) = self.codebase_intelligence {
                task.semantic_embedding = intelligence.generate_task_embedding(&task).await.ok();
            }
        }
        
        // Add to knowledge graph if enabled
        if self.config.enable_knowledge_graph {
            if let Some(ref intelligence) = self.codebase_intelligence {
                task.knowledge_graph_id = intelligence.add_task_to_graph(&task).await.ok();
            }
        }
        
        // Save to database
        self.database.save_task(&task).await?;
        
        // Update in-memory cache
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task.id.clone(), task.clone());
        }
        
        // Send event
        self.event_manager.send_task_created(task, None)?;
        
        Ok(task.id)
    }
    
    /// Update an existing task
    pub async fn update_task(&self, task_id: &str, updates: TaskUpdate) -> Result<(), TaskManagerError> {
        let mut task = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id)
                .ok_or_else(|| TaskManagerError::TaskNotFound(task_id.to_string()))?
                .clone()
        };
        
        let old_status = task.status.clone();
        
        // Apply updates
        if let Some(title) = updates.title {
            task.title = title;
        }
        if let Some(description) = updates.description {
            task.description = description;
        }
        if let Some(status) = updates.status {
            task.status = status;
        }
        if let Some(priority) = updates.priority {
            task.priority = priority;
        }
        if let Some(assigned_to) = updates.assigned_to {
            task.assigned_to = assigned_to;
        }
        if let Some(due_date) = updates.due_date {
            task.due_date = due_date;
        }
        if let Some(progress) = updates.progress_percentage {
            task.progress_percentage = progress;
        }
        if let Some(tags) = updates.tags {
            task.tags = tags;
        }
        if let Some(components) = updates.components {
            task.components = components;
        }
        if let Some(files_affected) = updates.files_affected {
            task.files_affected = files_affected;
        }
        
        task.updated_at = current_timestamp();
        
        // Validate updated task
        self.validate_task(&task).await?;
        
        // Update semantic embedding if content changed
        if self.config.enable_semantic_search && updates.content_changed() {
            if let Some(ref intelligence) = self.codebase_intelligence {
                task.semantic_embedding = intelligence.generate_task_embedding(&task).await.ok();
            }
        }
        
        // Update knowledge graph if enabled
        if self.config.enable_knowledge_graph {
            if let Some(ref intelligence) = self.codebase_intelligence {
                intelligence.update_task_in_graph(&task).await.ok();
            }
        }
        
        // Save to database
        self.database.save_task(&task).await?;
        
        // Update in-memory cache
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task.id.clone(), task.clone());
        }
        
        // Send events
        if task.status != old_status {
            self.event_manager.send_status_change(task.id.clone(), old_status, task.status.clone())?;
        }
        self.event_manager.send_event(TaskEvent::TaskUpdated(task))?;
        
        Ok(())
    }
    
    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Result<Task, TaskManagerError> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id)
            .cloned()
            .ok_or_else(|| TaskManagerError::TaskNotFound(task_id.to_string()))
    }
    
    /// Delete a task
    pub async fn delete_task(&self, task_id: &str) -> Result<(), TaskManagerError> {
        // Check if task exists
        {
            let tasks = self.tasks.read().await;
            if !tasks.contains_key(task_id) {
                return Err(TaskManagerError::TaskNotFound(task_id.to_string()));
            }
        }
        
        // Remove from knowledge graph if enabled
        if self.config.enable_knowledge_graph {
            if let Some(ref intelligence) = self.codebase_intelligence {
                intelligence.remove_task_from_graph(task_id).await.ok();
            }
        }
        
        // Delete from database (cascades to related data)
        self.database.delete_task(task_id).await?;
        
        // Remove from in-memory cache
        {
            let mut tasks = self.tasks.write().await;
            tasks.remove(task_id);
        }
        
        // Send event
        self.event_manager.send_event(TaskEvent::TaskDeleted(task_id.to_string()))?;
        
        Ok(())
    }
    
    /// Search tasks
    pub async fn search_tasks(
        &self,
        query: &str,
        filter: Option<TaskFilter>,
        limit: usize,
    ) -> Result<Vec<TaskSearchResult>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        
        if self.config.enable_semantic_search {
            self.search_engine.semantic_search(query, &tasks, limit, filter).await
        } else {
            self.search_engine.text_search(query, &tasks, limit, filter).await
        }
    }
    
    /// Find related tasks
    pub async fn find_related_tasks(
        &self,
        task_id: &str,
        max_depth: u8,
    ) -> Result<Vec<Task>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        self.search_engine.find_related_tasks(task_id, &tasks, max_depth).await
    }
    
    /// Get task suggestions based on context
    pub async fn suggest_tasks(
        &self,
        context: &str,
        limit: usize,
    ) -> Result<Vec<TaskSearchResult>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        self.search_engine.suggest_tasks(context, &tasks, limit).await
    }
    
    /// Create a new project
    pub async fn create_project(&self, mut project: Project) -> Result<String, TaskManagerError> {
        project.created_at = current_timestamp();
        project.updated_at = project.created_at;
        
        if project.id.is_empty() {
            project.id = generate_id();
        }
        
        // Save to database
        self.database.save_project(&project).await?;
        
        // Update in-memory cache
        {
            let mut projects = self.projects.write().await;
            projects.insert(project.id.clone(), project.clone());
        }
        
        // Send event
        self.event_manager.send_event(TaskEvent::ProjectCreated(project))?;
        
        Ok(project.id)
    }
    
    /// Create a new epic
    pub async fn create_epic(&self, mut epic: Epic) -> Result<String, TaskManagerError> {
        epic.created_at = current_timestamp();
        epic.updated_at = epic.created_at;
        
        if epic.id.is_empty() {
            epic.id = generate_id();
        }
        
        // Validate project exists
        {
            let projects = self.projects.read().await;
            if !projects.contains_key(&epic.project_id) {
                return Err(TaskManagerError::ProjectNotFound(epic.project_id.clone()));
            }
        }
        
        // Save to database
        self.database.save_epic(&epic).await?;
        
        // Update in-memory cache
        {
            let mut epics = self.epics.write().await;
            epics.insert(epic.id.clone(), epic.clone());
        }
        
        // Send event
        self.event_manager.send_event(TaskEvent::EpicCreated(epic))?;
        
        Ok(epic.id)
    }
    
    /// Add task dependency
    pub async fn add_dependency(
        &self,
        task_id: &str,
        depends_on_task_id: &str,
        dependency_type: DependencyType,
    ) -> Result<String, TaskManagerError> {
        // Validate both tasks exist
        {
            let tasks = self.tasks.read().await;
            if !tasks.contains_key(task_id) {
                return Err(TaskManagerError::TaskNotFound(task_id.to_string()));
            }
            if !tasks.contains_key(depends_on_task_id) {
                return Err(TaskManagerError::TaskNotFound(depends_on_task_id.to_string()));
            }
        }
        
        // Check for circular dependencies
        if self.would_create_cycle(task_id, depends_on_task_id).await? {
            return Err(TaskManagerError::CircularDependency);
        }
        
        let dependency = TaskDependency {
            id: generate_id(),
            task_id: task_id.to_string(),
            depends_on_task_id: depends_on_task_id.to_string(),
            dependency_type,
            created_at: current_timestamp(),
            notes: None,
        };
        
        // Save to database
        self.database.save_dependency(&dependency).await?;
        
        // Update in-memory cache
        {
            let mut dependencies = self.dependencies.write().await;
            dependencies.insert(dependency.id.clone(), dependency.clone());
        }
        
        // Send event
        self.event_manager.send_event(TaskEvent::DependencyAdded(dependency.clone()))?;
        
        Ok(dependency.id)
    }
    
    /// Add comment to task
    pub async fn add_comment(&self, mut comment: TaskComment) -> Result<String, TaskManagerError> {
        // Validate task exists
        {
            let tasks = self.tasks.read().await;
            if !tasks.contains_key(&comment.task_id) {
                return Err(TaskManagerError::TaskNotFound(comment.task_id.clone()));
            }
        }
        
        comment.created_at = current_timestamp();
        
        if comment.id.is_empty() {
            comment.id = generate_id();
        }
        
        // Save to database
        self.database.save_comment(&comment).await?;
        
        // Send event
        self.event_manager.send_event(TaskEvent::TaskCommentAdded(comment.clone()))?;
        
        Ok(comment.id)
    }
    
    /// Get task analytics
    pub async fn get_analytics(&self) -> Result<TaskAnalytics, TaskManagerError> {
        let tasks = self.tasks.read().await;
        let projects = self.projects.read().await;
        
        // For now, we'll use empty time entries - in a full implementation,
        // you'd load these from the database
        let time_entries = vec![];
        
        Ok(self.analytics_engine.generate_analytics(&tasks, &projects, &time_entries))
    }
    
    /// Get project analytics
    pub async fn get_project_analytics(&self, project_id: &str) -> Result<Option<ProjectAnalytics>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        let projects = self.projects.read().await;
        let time_entries = vec![];
        
        Ok(self.analytics_engine.generate_project_analytics(project_id, &tasks, &projects, &time_entries))
    }
    
    /// Get task trends
    pub async fn get_trends(&self, days: u32) -> Result<TaskTrends, TaskManagerError> {
        let tasks = self.tasks.read().await;
        Ok(self.analytics_engine.generate_trends(&tasks, days))
    }
    
    /// Subscribe to task events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.event_manager.subscribe()
    }
    
    /// Get all tasks (for UI display)
    pub async fn get_all_tasks(&self) -> Result<Vec<Task>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values().cloned().collect())
    }
    
    /// Get all projects
    pub async fn get_all_projects(&self) -> Result<Vec<Project>, TaskManagerError> {
        let projects = self.projects.read().await;
        Ok(projects.values().cloned().collect())
    }
    
    /// Get tasks by project
    pub async fn get_tasks_by_project(&self, project_id: &str) -> Result<Vec<Task>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values()
            .filter(|task| task.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect())
    }
    
    /// Get tasks by assignee
    pub async fn get_tasks_by_assignee(&self, assignee: &str) -> Result<Vec<Task>, TaskManagerError> {
        let tasks = self.tasks.read().await;
        Ok(tasks.values()
            .filter(|task| task.assigned_to.as_ref() == Some(assignee))
            .cloned()
            .collect())
    }
    
    // Agent Coordination Methods
    
    /// Assign an agent to a task
    pub async fn assign_agent_to_task(&self, request: AgentAssignmentRequest) -> Result<String, TaskManagerError> {
        // Validate task exists
        {
            let tasks = self.tasks.read().await;
            if !tasks.contains_key(&request.task_id) {
                return Err(TaskManagerError::TaskNotFound(request.task_id.clone()));
            }
        }
        
        // Assign through coordinator
        let assignment_id = self.agent_coordinator.assign_agent(request.clone()).await?;
        
        // Update task with agent assignment
        let agent_assignment = AgentAssignment {
            agent_id: request.agent_id.clone(),
            agent_type: request.agent_type.clone(),
            model_info: request.model_info.clone(),
            assigned_at: current_timestamp(),
            status: AgentTaskStatus::Assigned,
            progress_notes: None,
            estimated_completion: request.estimated_duration.map(|d| current_timestamp() + (d as u64 * 60)),
        };
        
        // Add to task's active agents
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&request.task_id) {
                task.active_agents.push(agent_assignment.clone());
                task.collaboration_mode = request.collaboration_mode;
                task.updated_at = current_timestamp();
                
                // Save to database
                self.database.save_task(task).await?;
            }
        }
        
        Ok(assignment_id)
    }
    
    /// Update agent status on a task
    pub async fn update_agent_status(
        &self,
        task_id: &str,
        agent_id: &str,
        status: AgentTaskStatus,
        progress_notes: Option<String>,
    ) -> Result<(), TaskManagerError> {
        // Update through coordinator
        self.agent_coordinator.update_agent_status(task_id, agent_id, status.clone(), progress_notes.clone()).await?;
        
        // Update task's agent list
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(task_id) {
                // Update active agent status
                if let Some(agent) = task.active_agents.iter_mut().find(|a| a.agent_id == agent_id) {
                    agent.status = status.clone();
                    agent.progress_notes = progress_notes.clone();
                }
                
                // Add to agent history
                let activity = AgentActivity {
                    agent_id: agent_id.to_string(),
                    agent_type: task.active_agents.iter()
                        .find(|a| a.agent_id == agent_id)
                        .map(|a| a.agent_type.clone())
                        .unwrap_or_default(),
                    model_info: task.active_agents.iter()
                        .find(|a| a.agent_id == agent_id)
                        .map(|a| a.model_info.clone())
                        .unwrap_or_else(|| ModelInfo {
                            provider: "unknown".to_string(),
                            model_name: "unknown".to_string(),
                            version: None,
                            context_window: None,
                            icon_url: None,
                        }),
                    activity_type: match status {
                        AgentTaskStatus::Working => AgentActivityType::Started,
                        AgentTaskStatus::Completed => AgentActivityType::Completed,
                        AgentTaskStatus::Failed => AgentActivityType::Failed,
                        AgentTaskStatus::Paused => AgentActivityType::Paused,
                        AgentTaskStatus::Handed_Off => AgentActivityType::Handed_Off,
                        _ => AgentActivityType::Progress_Update,
                    },
                    timestamp: current_timestamp(),
                    description: progress_notes.unwrap_or_default(),
                    duration_seconds: None,
                    result: None,
                };
                
                task.agent_history.push(activity);
                task.updated_at = current_timestamp();
                
                // Save to database
                self.database.save_task(task).await?;
            }
        }
        
        Ok(())
    }
    
    /// Get active agents for a task
    pub async fn get_task_agents(&self, task_id: &str) -> Result<Vec<AgentAssignment>, TaskManagerError> {
        self.agent_coordinator.get_task_agents(task_id).await
    }
    
    /// Get agent activity summary for a task
    pub async fn get_agent_activity_summary(&self, task_id: &str) -> Result<AgentActivitySummary, TaskManagerError> {
        self.agent_coordinator.get_agent_activity_summary(task_id).await
    }
    
    /// Subscribe to agent coordination events
    pub fn subscribe_to_coordination_events(&self) -> broadcast::Receiver<CoordinationEvent> {
        self.agent_coordinator.subscribe_to_events()
    }
    
    /// Get all active agents across all tasks (for dashboard)
    pub async fn get_all_active_agents(&self) -> Result<HashMap<String, Vec<AgentAssignment>>, TaskManagerError> {
        self.agent_coordinator.get_all_active_agents().await
    }
    
    /// Detect conflicts between agents
    pub async fn detect_agent_conflicts(&self, task_id: &str) -> Result<Vec<crate::task_manager::agent_coordination::AgentConflict>, TaskManagerError> {
        self.agent_coordinator.detect_conflicts(task_id).await
    }
    
    /// Validate task data
    async fn validate_task(&self, task: &Task) -> Result<(), TaskManagerError> {
        if task.title.trim().is_empty() {
            return Err(TaskManagerError::InvalidInput("Task title cannot be empty".to_string()));
        }
        
        if task.progress_percentage > 100 {
            return Err(TaskManagerError::InvalidInput("Progress cannot exceed 100%".to_string()));
        }
        
        // Validate project exists if specified
        if let Some(ref project_id) = task.project_id {
            let projects = self.projects.read().await;
            if !projects.contains_key(project_id) {
                return Err(TaskManagerError::ProjectNotFound(project_id.clone()));
            }
        }
        
        // Validate epic exists if specified
        if let Some(ref epic_id) = task.epic_id {
            let epics = self.epics.read().await;
            if !epics.contains_key(epic_id) {
                return Err(TaskManagerError::EpicNotFound(epic_id.clone()));
            }
        }
        
        Ok(())
    }
    
    /// Check if adding a dependency would create a cycle
    async fn would_create_cycle(&self, task_id: &str, depends_on_task_id: &str) -> Result<bool, TaskManagerError> {
        // Simple cycle detection - in a full implementation, you'd do a proper graph traversal
        let dependencies = self.dependencies.read().await;
        
        // Check direct cycle
        if task_id == depends_on_task_id {
            return Ok(true);
        }
        
        // Check if depends_on_task_id already depends on task_id (direct or indirect)
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![depends_on_task_id];
        
        while let Some(current_task) = stack.pop() {
            if visited.contains(current_task) {
                continue;
            }
            visited.insert(current_task);
            
            if current_task == task_id {
                return Ok(true);
            }
            
            // Find all tasks that current_task depends on
            for dep in dependencies.values() {
                if dep.task_id == current_task {
                    stack.push(&dep.depends_on_task_id);
                }
            }
        }
        
        Ok(false)
    }
}

/// Task update structure for partial updates
#[derive(Debug, Default)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub assigned_to: Option<Option<String>>,
    pub due_date: Option<Option<u64>>,
    pub progress_percentage: Option<u8>,
    pub tags: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub files_affected: Option<Vec<String>>,
}

impl TaskUpdate {
    /// Check if any content fields were changed (for semantic embedding updates)
    pub fn content_changed(&self) -> bool {
        self.title.is_some() || self.description.is_some() || self.tags.is_some()
    }
}
