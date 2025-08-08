use crate::task_manager::core::*;
use crate::task_manager::events::TaskUpdateEvent;
use sqlx::{SqlitePool, Row};
use std::collections::HashMap;
use serde_json;
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;

/// Enhanced database operations for PA system and task manager
pub struct TaskDatabase {
    pool: SqlitePool,
    // Real-time update broadcasting
    update_broadcaster: broadcast::Sender<TaskUpdateEvent>,
    // PA system integration
    pa_agent_id: Option<String>,
}

impl TaskDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        let (update_broadcaster, _) = broadcast::channel(1000);
        Self {
            pool,
            update_broadcaster,
            pa_agent_id: None,
        }
    }
    
    /// Initialize database schema
    pub async fn initialize_schema(&self) -> Result<(), TaskManagerError> {
        // Create projects table
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                due_date INTEGER,
                start_date INTEGER,
                owner TEXT NOT NULL,
                team_members TEXT NOT NULL,
                tags TEXT NOT NULL,
                repository_url TEXT,
                documentation_url TEXT,
                metadata TEXT NOT NULL
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create epics table
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS epics (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                project_id TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                due_date INTEGER,
                owner TEXT NOT NULL,
                tags TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects (id)
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create tasks table with all fields including agent tracking
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority INTEGER NOT NULL,
                task_type TEXT NOT NULL,
                parent_id TEXT,
                project_id TEXT,
                epic_id TEXT,
                assigned_to TEXT,
                created_by TEXT NOT NULL,
                reviewers TEXT NOT NULL,
                active_agents TEXT NOT NULL DEFAULT '[]',
                agent_history TEXT NOT NULL DEFAULT '[]',
                collaboration_mode TEXT NOT NULL DEFAULT 'Single',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                due_date INTEGER,
                start_date INTEGER,
                estimated_hours REAL,
                actual_hours REAL,
                progress_percentage INTEGER NOT NULL DEFAULT 0,
                completion_notes TEXT,
                blockers TEXT NOT NULL,
                tags TEXT NOT NULL,
                labels TEXT NOT NULL,
                components TEXT NOT NULL,
                files_affected TEXT NOT NULL,
                metadata TEXT NOT NULL,
                semantic_embedding BLOB,
                knowledge_graph_id TEXT,
                FOREIGN KEY (project_id) REFERENCES projects (id),
                FOREIGN KEY (epic_id) REFERENCES epics (id),
                FOREIGN KEY (parent_id) REFERENCES tasks (id)
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create task dependencies table
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS task_dependencies (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                depends_on_task_id TEXT NOT NULL,
                dependency_type TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                notes TEXT,
                FOREIGN KEY (task_id) REFERENCES tasks (id),
                FOREIGN KEY (depends_on_task_id) REFERENCES tasks (id)
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create task comments table
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS task_comments (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                author TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER,
                comment_type TEXT NOT NULL,
                mentions TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks (id)
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create time entries table
        sqlx::query!(
            "CREATE TABLE IF NOT EXISTS time_entries (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration_hours REAL,
                description TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks (id)
            )"
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        // Create indexes for performance
        self.create_indexes().await?;
        
        Ok(())
    }
    
    async fn create_indexes(&self) -> Result<(), TaskManagerError> {
        let indexes = vec![
            "CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_assigned_to ON tasks(assigned_to)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority)",
            "CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date)",
            "CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id ON task_dependencies(task_id)",
            "CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id)",
            "CREATE INDEX IF NOT EXISTS idx_task_comments_task_id ON task_comments(task_id)",
            "CREATE INDEX IF NOT EXISTS idx_time_entries_task_id ON time_entries(task_id)",
        ];
        
        for index_sql in indexes {
            sqlx::query(index_sql)
                .execute(&self.pool)
                .await
                .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    /// Save project to database
    pub async fn save_project(&self, project: &Project) -> Result<(), TaskManagerError> {
        // Store serialized values to avoid temporary value drops
        let status_json = serde_json::to_string(&project.status).unwrap();
        let team_members_json = serde_json::to_string(&project.team_members).unwrap();
        let tags_json = serde_json::to_string(&project.tags).unwrap();
        let metadata_json = serde_json::to_string(&project.metadata).unwrap();
        let due_date_i64 = project.due_date.map(|d| d as i64);
        let start_date_i64 = project.start_date.map(|d| d as i64);

        sqlx::query!(
            "INSERT OR REPLACE INTO projects (
                id, name, description, status, created_at, updated_at, due_date, start_date,
                owner, team_members, tags, repository_url, documentation_url, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            project.id,
            project.name,
            project.description,
            status_json,
            project.created_at as i64,
            project.updated_at as i64,
            due_date_i64,
            start_date_i64,
            project.owner,
            team_members_json,
            tags_json,
            project.repository_url,
            project.documentation_url,
            metadata_json
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;

        Ok(())
    }
    
    /// Save epic to database
    pub async fn save_epic(&self, epic: &Epic) -> Result<(), TaskManagerError> {
        // Store serialized values to avoid temporary value drops
        let status_json = serde_json::to_string(&epic.status).unwrap();
        let tags_json = serde_json::to_string(&epic.tags).unwrap();

        sqlx::query!(
            "INSERT OR REPLACE INTO epics (
                id, title, description, project_id, status, created_at, updated_at, due_date, owner, tags
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            epic.id,
            epic.title,
            epic.description,
            epic.project_id,
            status_json,
            epic.created_at as i64,
            epic.updated_at as i64,
            epic.due_date.map(|d| d as i64),
            epic.owner,
            tags_json
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Save task to database
    pub async fn save_task(&self, task: &Task) -> Result<(), TaskManagerError> {
        // Store serialized values to avoid temporary value drops
        let semantic_embedding_blob = task.semantic_embedding.as_ref()
            .map(|emb| bincode::serialize(emb).unwrap_or_default());
        let status_json = serde_json::to_string(&task.status).unwrap();
        let task_type_json = serde_json::to_string(&task.task_type).unwrap();
        let reviewers_json = serde_json::to_string(&task.reviewers).unwrap();
        let active_agents_json = serde_json::to_string(&task.active_agents).unwrap();
        let agent_history_json = serde_json::to_string(&task.agent_history).unwrap();
        let collaboration_mode_json = serde_json::to_string(&task.collaboration_mode).unwrap();
        let blockers_json = serde_json::to_string(&task.blockers).unwrap();
        let tags_json = serde_json::to_string(&task.tags).unwrap();
        let labels_json = serde_json::to_string(&task.labels).unwrap();
        let components_json = serde_json::to_string(&task.components).unwrap();
        let files_affected_json = serde_json::to_string(&task.files_affected).unwrap();
        let metadata_json = serde_json::to_string(&task.metadata).unwrap();

        sqlx::query!(
            "INSERT OR REPLACE INTO tasks (
                id, title, description, status, priority, task_type, parent_id, project_id, epic_id,
                assigned_to, created_by, reviewers, active_agents, agent_history, collaboration_mode,
                created_at, updated_at, due_date, start_date, estimated_hours, actual_hours,
                completion_percentage, completion_notes, blockers, tags, labels, components,
                files_affected, metadata, semantic_embedding, knowledge_graph_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            task.id,
            task.title,
            task.description,
            status_json,
            task.priority as i32,
            task_type_json,
            task.parent_id,
            task.project_id,
            task.epic_id,
            task.assigned_to,
            task.created_by,
            reviewers_json,
            active_agents_json,
            agent_history_json,
            collaboration_mode_json,
            task.created_at as i64,
            task.updated_at as i64,
            task.due_date.map(|d| d as i64),
            task.start_date.map(|d| d as i64),
            task.estimated_hours,
            task.actual_hours,
            task.progress_percentage as i32,
            task.completion_notes,
            blockers_json,
            tags_json,
            labels_json,
            components_json,
            files_affected_json,
            metadata_json,
            semantic_embedding_blob,
            task.knowledge_graph_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Save task dependency
    pub async fn save_dependency(&self, dependency: &TaskDependency) -> Result<(), TaskManagerError> {
        // Store serialized values to avoid temporary value drops
        let dependency_type_json = serde_json::to_string(&dependency.dependency_type).unwrap();

        sqlx::query!(
            "INSERT OR REPLACE INTO task_dependencies (
                id, task_id, depends_on_task_id, dependency_type, created_at, notes
            ) VALUES (?, ?, ?, ?, ?, ?)",
            dependency.id,
            dependency.task_id,
            dependency.depends_on_task_id,
            dependency_type_json,
            dependency.created_at as i64,
            dependency.notes
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;

        Ok(())
    }
    
    /// Save task comment
    pub async fn save_comment(&self, comment: &TaskComment) -> Result<(), TaskManagerError> {
        // Store serialized values to avoid temporary value drops
        let comment_type_json = serde_json::to_string(&comment.comment_type).unwrap();
        let mentions_json = serde_json::to_string(&comment.mentions).unwrap();

        sqlx::query!(
            "INSERT OR REPLACE INTO task_comments (
                id, task_id, author, content, created_at, updated_at, comment_type, mentions
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            comment.id,
            comment.task_id,
            comment.author,
            comment.content,
            comment.created_at as i64,
            comment.updated_at.map(|t| t as i64),
            comment_type_json,
            mentions_json
        )
        .execute(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;

        Ok(())
    }
    
    /// Load all projects from database
    pub async fn load_projects(&self) -> Result<HashMap<String, Project>, TaskManagerError> {
        let rows = sqlx::query!("SELECT * FROM projects")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        let mut projects = HashMap::new();
        for row in rows {
            let project = Project {
                id: row.id,
                name: row.name,
                description: row.description,
                status: serde_json::from_str(&row.status).unwrap_or(ProjectStatus::Planning),
                created_at: row.created_at as u64,
                updated_at: row.updated_at as u64,
                due_date: row.due_date.map(|d| d as u64),
                start_date: row.start_date.map(|d| d as u64),
                owner: row.owner,
                team_members: serde_json::from_str(&row.team_members).unwrap_or_default(),
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
                repository_url: row.repository_url,
                documentation_url: row.documentation_url,
                metadata: serde_json::from_str(&row.metadata).unwrap_or_default(),
            };
            projects.insert(project.id.clone(), project);
        }
        
        Ok(projects)
    }
    
    /// Load all epics from database
    pub async fn load_epics(&self) -> Result<HashMap<String, Epic>, TaskManagerError> {
        let rows = sqlx::query!("SELECT * FROM epics")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        let mut epics = HashMap::new();
        for row in rows {
            let epic = Epic {
                id: row.id,
                title: row.title,
                description: row.description,
                project_id: row.project_id,
                status: serde_json::from_str(&row.status).unwrap_or(EpicStatus::Planning),
                created_at: row.created_at as u64,
                updated_at: row.updated_at as u64,
                due_date: row.due_date.map(|d| d as u64),
                owner: row.owner,
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
            };
            epics.insert(epic.id.clone(), epic);
        }
        
        Ok(epics)
    }
    
    /// Load all tasks from database
    pub async fn load_tasks(&self) -> Result<HashMap<String, Task>, TaskManagerError> {
        let rows = sqlx::query!("SELECT * FROM tasks")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        let mut tasks = HashMap::new();
        for row in rows {
            let semantic_embedding: Option<Vec<f32>> = row.semantic_embedding
                .map(|blob| bincode::deserialize(&blob).ok()).flatten();
            
            let task = Task {
                id: row.id,
                title: row.title,
                description: row.description,
                status: serde_json::from_str(&row.status).unwrap_or(TaskStatus::Todo),
                priority: match row.priority {
                    1 => TaskPriority::Low,
                    2 => TaskPriority::Medium,
                    3 => TaskPriority::High,
                    4 => TaskPriority::Critical,
                    _ => TaskPriority::Medium,
                },
                task_type: serde_json::from_str(&row.task_type).unwrap_or(TaskType::Feature),
                parent_id: row.parent_id,
                project_id: row.project_id,
                epic_id: row.epic_id,
                assigned_to: row.assigned_to,
                created_by: row.created_by,
                reviewers: serde_json::from_str(&row.reviewers).unwrap_or_default(),
                created_at: row.created_at as u64,
                updated_at: row.updated_at as u64,
                due_date: row.due_date.map(|d| d as u64),
                start_date: row.start_date.map(|d| d as u64),
                estimated_hours: row.estimated_hours,
                actual_hours: row.actual_hours,
                progress_percentage: 0, // Default value - database schema needs progress_percentage column
                completion_notes: row.completion_notes,
                blockers: serde_json::from_str(&row.blockers).unwrap_or_default(),
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
                labels: serde_json::from_str(&row.labels).unwrap_or_default(),
                components: serde_json::from_str(&row.components).unwrap_or_default(),
                files_affected: serde_json::from_str(&row.files_affected).unwrap_or_default(),
                metadata: serde_json::from_str(&row.metadata).unwrap_or_default(),
                semantic_embedding,
                knowledge_graph_id: row.knowledge_graph_id,
                // Multi-agent collaboration fields
                active_agents: Vec::new(),
                agent_history: Vec::new(),
                collaboration_mode: CollaborationMode::default(),
            };
            tasks.insert(task.id.clone(), task);
        }
        
        Ok(tasks)
    }
    
    /// Load all dependencies from database
    pub async fn load_dependencies(&self) -> Result<HashMap<String, TaskDependency>, TaskManagerError> {
        let rows = sqlx::query!("SELECT * FROM task_dependencies")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        let mut dependencies = HashMap::new();
        for row in rows {
            let dependency = TaskDependency {
                id: row.id,
                task_id: row.task_id,
                depends_on_task_id: row.depends_on_task_id,
                dependency_type: serde_json::from_str(&row.dependency_type).unwrap_or(DependencyType::FinishToStart),
                required_output: None, // Not stored in database yet
                created_at: row.created_at as u64,
                notes: row.notes,
            };
            dependencies.insert(row.id, dependency);
        }
        
        Ok(dependencies)
    }
    
    /// Get task comments
    pub async fn get_task_comments(&self, task_id: &str) -> Result<Vec<TaskComment>, TaskManagerError> {
        let rows = sqlx::query!("SELECT * FROM task_comments WHERE task_id = ? ORDER BY created_at ASC", task_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        let mut comments = Vec::new();
        for row in rows {
            let comment = TaskComment {
                id: row.id,
                task_id: row.task_id,
                author: row.author,
                content: row.content,
                created_at: row.created_at as u64,
                updated_at: row.updated_at.map(|t| t as u64),
                comment_type: serde_json::from_str(&row.comment_type).unwrap_or(CommentType::General),
                mentions: serde_json::from_str(&row.mentions).unwrap_or_default(),
            };
            comments.push(comment);
        }
        
        Ok(comments)
    }
    
    /// Delete task and related data
    pub async fn delete_task(&self, task_id: &str) -> Result<(), TaskManagerError> {
        // Delete in order due to foreign key constraints
        sqlx::query!("DELETE FROM task_comments WHERE task_id = ?", task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        sqlx::query!("DELETE FROM time_entries WHERE task_id = ?", task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        sqlx::query!("DELETE FROM task_dependencies WHERE task_id = ? OR depends_on_task_id = ?", task_id, task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        sqlx::query!("DELETE FROM tasks WHERE id = ?", task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Text search in tasks (SQLite fallback)
    pub async fn text_search_tasks(&self, query: &str, limit: i64) -> Result<Vec<Task>, TaskManagerError> {
        let search_pattern = format!("%{}%", query);
        
        let rows = sqlx::query!(
            "SELECT * FROM tasks 
             WHERE title LIKE ? OR description LIKE ? 
             ORDER BY 
                CASE 
                    WHEN title LIKE ? THEN 1 
                    WHEN description LIKE ? THEN 2 
                    ELSE 3 
                END
             LIMIT ?",
            search_pattern, search_pattern, search_pattern, search_pattern, limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| TaskManagerError::DatabaseError(e.to_string()))?;
        
        let mut tasks = Vec::new();
        for row in rows {
            let semantic_embedding = row.semantic_embedding
                .map(|blob| bincode::deserialize(&blob).ok()).flatten();

            let task = Task {
                id: row.id,
                title: row.title,
                description: row.description,
                status: serde_json::from_str(&row.status).unwrap_or(TaskStatus::Todo),
                priority: match row.priority {
                    1 => TaskPriority::Low,
                    2 => TaskPriority::Medium,
                    3 => TaskPriority::High,
                    4 => TaskPriority::Critical,
                    _ => TaskPriority::Medium,
                },
                task_type: serde_json::from_str(&row.task_type).unwrap_or(TaskType::Feature),
                parent_id: row.parent_id,
                project_id: row.project_id,
                epic_id: row.epic_id,
                assigned_to: row.assigned_to,
                created_by: row.created_by,
                reviewers: serde_json::from_str(&row.reviewers).unwrap_or_default(),
                created_at: row.created_at as u64,
                updated_at: row.updated_at as u64,
                due_date: row.due_date.map(|d| d as u64),
                start_date: row.start_date.map(|d| d as u64),
                estimated_hours: row.estimated_hours,
                actual_hours: row.actual_hours,
                progress_percentage: 0, // Default value - database schema needs progress_percentage column
                completion_notes: row.completion_notes,
                blockers: serde_json::from_str(&row.blockers).unwrap_or_default(),
                tags: serde_json::from_str(&row.tags).unwrap_or_default(),
                labels: serde_json::from_str(&row.labels).unwrap_or_default(),
                components: serde_json::from_str(&row.components).unwrap_or_default(),
                files_affected: serde_json::from_str(&row.files_affected).unwrap_or_default(),
                metadata: serde_json::from_str(&row.metadata).unwrap_or_default(),
                semantic_embedding,
                knowledge_graph_id: row.knowledge_graph_id,
                // Multi-agent collaboration fields
                active_agents: Vec::new(),
                agent_history: Vec::new(),
                collaboration_mode: CollaborationMode::default(),
            };
            tasks.push(task);
        }
        
        Ok(tasks)
    }
}
