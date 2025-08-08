use crate::task_manager::{
    core::*,
    manager::{TaskManager, TaskUpdate},
    search::{TaskFilter, TaskSearchResult},
    agent_coordination::{AgentAssignmentRequest, AssignmentPriority},
};
use crate::agent_runtime::{Agent, AgentCapability, AgentError, AgentResult, AgentContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Task/Planner Agent for AI-driven task management
pub struct TaskPlannerAgent {
    id: String,
    task_manager: Arc<RwLock<Option<Arc<TaskManager>>>>,
    planning_context: Arc<RwLock<PlanningContext>>,
}

#[derive(Debug, Default)]
struct PlanningContext {
    active_plans: HashMap<String, ProjectPlan>,
    user_preferences: PlannerPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPlan {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub phases: Vec<PlanPhase>,
    pub estimated_duration: Option<u32>,
    pub created_at: u64,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPhase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tasks: Vec<String>,
    pub estimated_duration: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanStatus {
    Draft,
    Active,
    Completed,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PlannerPreferences {
    default_task_priority: TaskPriority,
    auto_assign_tasks: bool,
    planning_style: PlanningStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PlanningStyle {
    Agile,
    Waterfall,
    Flexible,
}

impl Default for PlanningStyle {
    fn default() -> Self {
        Self::Agile
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningRequest {
    pub request_type: PlanningRequestType,
    pub context: String,
    pub project_id: Option<String>,
    pub constraints: PlanningConstraints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanningRequestType {
    CreateProjectPlan,
    BreakdownTask(String),
    SuggestNextTasks,
    OptimizeSchedule,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlanningConstraints {
    pub deadline: Option<u64>,
    pub available_team_members: Vec<String>,
    pub budget_hours: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningResult {
    pub success: bool,
    pub plan: Option<ProjectPlan>,
    pub tasks_created: Vec<String>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

impl TaskPlannerAgent {
    pub fn new() -> Self {
        Self {
            id: "task_planner_agent".to_string(),
            task_manager: Arc::new(RwLock::new(None)),
            planning_context: Arc::new(RwLock::new(PlanningContext::default())),
        }
    }
    
    pub async fn set_task_manager(&self, task_manager: Arc<TaskManager>) {
        let mut tm = self.task_manager.write().await;
        *tm = Some(task_manager);
    }
    
    pub async fn create_project_plan(&self, request: PlanningRequest) -> Result<PlanningResult, AgentError> {
        let task_manager = self.get_task_manager().await?;
        let plan = self.generate_project_plan(&request).await?;
        let tasks_created = self.create_plan_tasks(&plan, &task_manager).await?;
        
        {
            let mut context = self.planning_context.write().await;
            context.active_plans.insert(plan.id.clone(), plan.clone());
        }
        
        Ok(PlanningResult {
            success: true,
            plan: Some(plan),
            tasks_created,
            recommendations: vec!["Plan created successfully".to_string()],
            warnings: vec![],
        })
    }
    
    pub async fn breakdown_task(&self, task_id: &str) -> Result<PlanningResult, AgentError> {
        let task_manager = self.get_task_manager().await?;
        let task = task_manager.get_task(task_id).await
            .map_err(|e| AgentError::TaskNotFound(e.to_string()))?;
        
        let subtasks = self.generate_subtasks(&task).await?;
        let mut tasks_created = Vec::new();
        
        for subtask in subtasks {
            let task_id = task_manager.create_task(subtask).await
                .map_err(|e| AgentError::DatabaseError(e.to_string()))?;
            tasks_created.push(task_id);
        }
        
        Ok(PlanningResult {
            success: true,
            plan: None,
            tasks_created,
            recommendations: vec!["Task broken down successfully".to_string()],
            warnings: vec![],
        })
    }
    
    pub async fn suggest_next_tasks(&self, context: &str, limit: usize) -> Result<Vec<TaskSearchResult>, AgentError> {
        let task_manager = self.get_task_manager().await?;
        task_manager.suggest_tasks(context, limit).await
            .map_err(|e| AgentError::SearchError(e.to_string()))
    }
    
    pub async fn auto_create_tasks_from_code(&self, file_path: &str, analysis: &str) -> Result<Vec<String>, AgentError> {
        let task_manager = self.get_task_manager().await?;
        let task_opportunities = self.analyze_code_for_tasks(file_path, analysis).await?;
        let mut created_tasks = Vec::new();
        
        for opportunity in task_opportunities {
            let task = Task {
                id: generate_id(),
                title: opportunity.title,
                description: Some(opportunity.description),
                status: TaskStatus::Todo,
                priority: opportunity.priority,
                task_type: opportunity.task_type,
                parent_id: None,
                project_id: None,
                epic_id: None,
                assigned_to: None,
                created_by: self.id.clone(),
                reviewers: vec![],
                active_agents: vec![],
                agent_history: vec![],
                collaboration_mode: CollaborationMode::Single,
                created_at: current_timestamp(),
                updated_at: current_timestamp(),
                due_date: None,
                start_date: None,
                estimated_hours: opportunity.estimated_hours,
                actual_hours: None,
                progress_percentage: 0,
                completion_notes: None,
                blockers: vec![],
                tags: opportunity.tags,
                labels: vec!["auto-generated".to_string()],
                components: vec![],
                files_affected: vec![file_path.to_string()],
                metadata: HashMap::new(),
                semantic_embedding: None,
                knowledge_graph_id: None,
            };
            
            let task_id = task_manager.create_task(task).await
                .map_err(|e| AgentError::DatabaseError(e.to_string()))?;
            created_tasks.push(task_id);
        }
        
        Ok(created_tasks)
    }
    
    async fn get_task_manager(&self) -> Result<Arc<TaskManager>, AgentError> {
        let tm = self.task_manager.read().await;
        tm.as_ref()
            .cloned()
            .ok_or_else(|| AgentError::InvalidTask("Task manager not initialized".to_string()))
    }
    
    async fn generate_project_plan(&self, request: &PlanningRequest) -> Result<ProjectPlan, AgentError> {
        let plan_id = generate_id();
        let current_time = current_timestamp();
        
        let phases = vec![
            PlanPhase {
                id: generate_id(),
                name: "Planning & Design".to_string(),
                description: "Define requirements and create system design".to_string(),
                tasks: vec![],
                estimated_duration: Some(7),
            },
            PlanPhase {
                id: generate_id(),
                name: "Core Implementation".to_string(),
                description: "Develop main features and functionality".to_string(),
                tasks: vec![],
                estimated_duration: Some(14),
            },
            PlanPhase {
                id: generate_id(),
                name: "Testing & QA".to_string(),
                description: "Comprehensive testing and quality assurance".to_string(),
                tasks: vec![],
                estimated_duration: Some(7),
            },
        ];
        
        Ok(ProjectPlan {
            id: plan_id,
            project_id: request.project_id.clone().unwrap_or_else(|| generate_id()),
            name: "AI-Generated Project Plan".to_string(),
            description: request.context.clone(),
            phases,
            estimated_duration: Some(28),
            created_at: current_time,
            status: PlanStatus::Draft,
        })
    }
    
    async fn create_plan_tasks(&self, plan: &ProjectPlan, task_manager: &TaskManager) -> Result<Vec<String>, AgentError> {
        let mut created_tasks = Vec::new();
        
        for phase in &plan.phases {
            let phase_tasks = self.generate_phase_tasks(phase, &plan.project_id).await?;
            
            for task in phase_tasks {
                let task_id = task_manager.create_task(task).await
                    .map_err(|e| AgentError::DatabaseError(e.to_string()))?;
                created_tasks.push(task_id);
            }
        }
        
        Ok(created_tasks)
    }
    
    async fn generate_phase_tasks(&self, phase: &PlanPhase, project_id: &str) -> Result<Vec<Task>, AgentError> {
        let current_time = current_timestamp();
        let mut tasks = Vec::new();
        
        match phase.name.as_str() {
            "Planning & Design" => {
                tasks.push(self.create_task_template(
                    "Define project requirements",
                    "Gather and document detailed project requirements",
                    TaskType::Planning,
                    TaskPriority::High,
                    Some(8.0),
                    project_id,
                    vec!["planning".to_string(), "requirements".to_string()],
                    current_time,
                ));
            },
            "Core Implementation" => {
                tasks.push(self.create_task_template(
                    "Implement core functionality",
                    "Develop the main features and core business logic",
                    TaskType::Feature,
                    TaskPriority::High,
                    Some(40.0),
                    project_id,
                    vec!["development".to_string(), "core".to_string()],
                    current_time,
                ));
            },
            "Testing & QA" => {
                tasks.push(self.create_task_template(
                    "Write comprehensive tests",
                    "Create unit, integration, and end-to-end tests",
                    TaskType::Testing,
                    TaskPriority::Medium,
                    Some(24.0),
                    project_id,
                    vec!["testing".to_string(), "qa".to_string()],
                    current_time,
                ));
            },
            _ => {
                tasks.push(self.create_task_template(
                    &format!("Complete {}", phase.name),
                    &phase.description,
                    TaskType::Feature,
                    TaskPriority::Medium,
                    Some(16.0),
                    project_id,
                    vec!["development".to_string()],
                    current_time,
                ));
            }
        }
        
        Ok(tasks)
    }
    
    fn create_task_template(
        &self,
        title: &str,
        description: &str,
        task_type: TaskType,
        priority: TaskPriority,
        estimated_hours: Option<f32>,
        project_id: &str,
        tags: Vec<String>,
        current_time: u64,
    ) -> Task {
        Task {
            id: generate_id(),
            title: title.to_string(),
            description: Some(description.to_string()),
            status: TaskStatus::Todo,
            priority,
            task_type,
            parent_id: None,
            project_id: Some(project_id.to_string()),
            epic_id: None,
            assigned_to: None,
            created_by: self.id.clone(),
            reviewers: vec![],
            active_agents: vec![],
            agent_history: vec![],
            collaboration_mode: CollaborationMode::Single,
            created_at: current_time,
            updated_at: current_time,
            due_date: None,
            start_date: None,
            estimated_hours,
            actual_hours: None,
            progress_percentage: 0,
            completion_notes: None,
            blockers: vec![],
            tags,
            labels: vec!["auto-generated".to_string()],
            components: vec![],
            files_affected: vec![],
            metadata: HashMap::new(),
            semantic_embedding: None,
            knowledge_graph_id: None,
        }
    }
    
    async fn generate_subtasks(&self, parent_task: &Task) -> Result<Vec<Task>, AgentError> {
        let current_time = current_timestamp();
        let mut subtasks = Vec::new();
        
        for i in 1..=4 {
            let subtask = Task {
                id: generate_id(),
                title: format!("{} - Part {}", parent_task.title, i),
                description: Some(format!("Subtask {} of {}", i, parent_task.title)),
                status: TaskStatus::Todo,
                priority: parent_task.priority,
                task_type: parent_task.task_type.clone(),
                parent_id: Some(parent_task.id.clone()),
                project_id: parent_task.project_id.clone(),
                epic_id: parent_task.epic_id.clone(),
                assigned_to: None,
                created_by: self.id.clone(),
                reviewers: vec![],
                active_agents: vec![],
                agent_history: vec![],
                collaboration_mode: CollaborationMode::Single,
                created_at: current_time,
                updated_at: current_time,
                due_date: parent_task.due_date,
                start_date: None,
                estimated_hours: parent_task.estimated_hours.map(|h| h / 4.0),
                actual_hours: None,
                progress_percentage: 0,
                completion_notes: None,
                blockers: vec![],
                tags: parent_task.tags.clone(),
                labels: vec!["subtask".to_string(), "auto-generated".to_string()],
                components: parent_task.components.clone(),
                files_affected: parent_task.files_affected.clone(),
                metadata: HashMap::new(),
                semantic_embedding: None,
                knowledge_graph_id: None,
            };
            subtasks.push(subtask);
        }
        
        Ok(subtasks)
    }
    
    async fn analyze_code_for_tasks(&self, file_path: &str, analysis: &str) -> Result<Vec<TaskOpportunity>, AgentError> {
        // Simplified task opportunity detection
        let mut opportunities = Vec::new();
        
        if analysis.contains("TODO") || analysis.contains("FIXME") {
            opportunities.push(TaskOpportunity {
                title: format!("Address TODO/FIXME in {}", file_path),
                description: "Review and resolve TODO/FIXME comments in code".to_string(),
                priority: TaskPriority::Medium,
                task_type: TaskType::Bug,
                estimated_hours: Some(2.0),
                tags: vec!["maintenance".to_string(), "code-review".to_string()],
            });
        }
        
        if analysis.contains("deprecated") || analysis.contains("legacy") {
            opportunities.push(TaskOpportunity {
                title: format!("Refactor deprecated code in {}", file_path),
                description: "Update deprecated code to use modern alternatives".to_string(),
                priority: TaskPriority::Low,
                task_type: TaskType::Refactor,
                estimated_hours: Some(4.0),
                tags: vec!["refactor".to_string(), "modernization".to_string()],
            });
        }
        
        Ok(opportunities)
    }
}

#[derive(Debug, Clone)]
struct TaskOpportunity {
    title: String,
    description: String,
    priority: TaskPriority,
    task_type: TaskType,
    estimated_hours: Option<f32>,
    tags: Vec<String>,
}

#[async_trait]
impl Agent for TaskPlannerAgent {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn capabilities(&self) -> Vec<AgentCapability> {
        vec![
            AgentCapability::TaskPlanning,
            AgentCapability::ProjectManagement,
            AgentCapability::TaskBreakdown,
            AgentCapability::CodeAnalysis,
        ]
    }
    
    async fn execute_task(&self, context: AgentContext) -> AgentResult<String> {
        match context.task_type.as_str() {
            "create_plan" => {
                let request = PlanningRequest {
                    request_type: PlanningRequestType::CreateProjectPlan,
                    context: context.input.clone(),
                    project_id: None,
                    constraints: PlanningConstraints::default(),
                };
                
                match self.create_project_plan(request).await {
                    Ok(result) => Ok(format!("Created plan with {} tasks", result.tasks_created.len())),
                    Err(e) => Err(AgentError::ExecutionError(format!("Failed to create plan: {}", e))),
                }
            },
            "breakdown_task" => {
                if let Some(task_id) = context.metadata.get("task_id") {
                    match self.breakdown_task(task_id).await {
                        Ok(result) => Ok(format!("Broke down task into {} subtasks", result.tasks_created.len())),
                        Err(e) => Err(AgentError::ExecutionError(format!("Failed to breakdown task: {}", e))),
                    }
                } else {
                    Err(AgentError::ExecutionError("No task_id provided for breakdown".to_string()))
                }
            },
            _ => Err(AgentError::ExecutionError(format!("Unknown task type: {}", context.task_type))),
        }
    }
}
