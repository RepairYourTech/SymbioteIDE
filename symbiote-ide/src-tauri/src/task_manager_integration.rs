use crate::task_manager::{TaskManager, AgentAssignmentRequest, AssignmentPriority, ModelInfo, CollaborationMode, AgentTaskStatus};
use crate::agent_orchestrator::AgentOrchestrator;
use crate::specialized_agents::AgentRegistry;
use crate::task_planner_agent::TaskPlannerAgent;
use crate::agent_runtime::{AgentError, AgentContext};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Integration layer between Task Manager and Agent system
pub struct TaskManagerIntegration {
    task_manager: Arc<TaskManager>,
    orchestrator: Arc<AgentOrchestrator>,
    agent_registry: Arc<AgentRegistry>,
    planner_agent: Arc<TaskPlannerAgent>,
}

/// Task assignment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignmentConfig {
    pub auto_assign: bool,
    pub preferred_agents: Vec<String>,
    pub collaboration_mode: CollaborationMode,
    pub max_parallel_agents: usize,
}

impl Default for TaskAssignmentConfig {
    fn default() -> Self {
        Self {
            auto_assign: true,
            preferred_agents: vec![],
            collaboration_mode: CollaborationMode::Single,
            max_parallel_agents: 3,
        }
    }
}

impl TaskManagerIntegration {
    pub fn new(
        task_manager: Arc<TaskManager>,
        orchestrator: Arc<AgentOrchestrator>,
        agent_registry: Arc<AgentRegistry>,
        planner_agent: Arc<TaskPlannerAgent>,
    ) -> Self {
        Self {
            task_manager,
            orchestrator,
            agent_registry,
            planner_agent,
        }
    }
    
    /// Initialize the integration
    pub async fn initialize(&self) -> Result<(), AgentError> {
        // Set task manager reference in planner agent
        self.planner_agent.set_task_manager(self.task_manager.clone()).await;
        
        // Set task manager reference in orchestrator
        self.orchestrator.set_task_manager(self.task_manager.clone()).await;
        
        Ok(())
    }
    
    /// Auto-assign agents to a task based on task type and requirements
    pub async fn auto_assign_agents(&self, task_id: &str, config: TaskAssignmentConfig) -> Result<Vec<String>, AgentError> {
        let task = self.task_manager.get_task(task_id).await
            .map_err(|e| AgentError::TaskNotFound(e.to_string()))?;
        
        // Determine best agents for this task
        let suitable_agents = self.find_suitable_agents(&task, &config).await?;
        
        let mut assigned_agents = Vec::new();
        
        for agent_info in suitable_agents {
            let assignment_request = AgentAssignmentRequest {
                task_id: task_id.to_string(),
                agent_id: agent_info.agent_id.clone(),
                agent_type: agent_info.agent_type.clone(),
                model_info: agent_info.model_info.clone(),
                collaboration_mode: config.collaboration_mode.clone(),
                priority: AssignmentPriority::Normal,
                estimated_duration: task.estimated_hours.map(|h| (h * 60.0) as u32),
            };
            
            let assignment_id = self.task_manager.assign_agent_to_task(assignment_request).await
                .map_err(|e| AgentError::AssignmentError(e.to_string()))?;
            
            assigned_agents.push(assignment_id);
            
            // Limit parallel agents
            if assigned_agents.len() >= config.max_parallel_agents {
                break;
            }
        }
        
        Ok(assigned_agents)
    }
    
    /// Execute a task using the orchestrator and update task manager
    pub async fn execute_task_with_agents(&self, task_id: &str) -> Result<String, AgentError> {
        let task = self.task_manager.get_task(task_id).await
            .map_err(|e| AgentError::TaskNotFound(e.to_string()))?;
        
        // Get assigned agents
        let assigned_agents = self.task_manager.get_task_agents(task_id).await
            .map_err(|e| AgentError::DatabaseError(e.to_string()))?;
        
        if assigned_agents.is_empty() {
            // Auto-assign if no agents assigned
            self.auto_assign_agents(task_id, TaskAssignmentConfig::default()).await?;
        }
        
        // Update agent status to working
        for agent in &assigned_agents {
            self.task_manager.update_agent_status(
                task_id,
                &agent.agent_id,
                AgentTaskStatus::Working,
                Some("Starting task execution".to_string()),
            ).await.map_err(|e| AgentError::DatabaseError(e.to_string()))?;
        }
        
        // Create agent context for execution
        let context = AgentContext {
            task_id: task_id.to_string(),
            task_type: format!("{:?}", task.task_type),
            input: task.description.unwrap_or_default(),
            metadata: std::collections::HashMap::new(),
            max_tokens: 4000,
        };
        
        // Execute through orchestrator (simplified - in practice would be more complex)
        let result = format!("Task {} executed with {} agents", task_id, assigned_agents.len());
        
        // Update agent status to completed
        for agent in &assigned_agents {
            self.task_manager.update_agent_status(
                task_id,
                &agent.agent_id,
                AgentTaskStatus::Completed,
                Some(result.clone()),
            ).await.map_err(|e| AgentError::DatabaseError(e.to_string()))?;
        }
        
        Ok(result)
    }
    
    /// Create a project plan using the planner agent
    pub async fn create_project_plan(&self, context: &str, project_id: Option<String>) -> Result<String, AgentError> {
        let request = crate::task_planner_agent::PlanningRequest {
            request_type: crate::task_planner_agent::PlanningRequestType::CreateProjectPlan,
            context: context.to_string(),
            project_id,
            constraints: crate::task_planner_agent::PlanningConstraints::default(),
        };
        
        let result = self.planner_agent.create_project_plan(request).await?;
        
        if result.success {
            Ok(format!("Created project plan with {} tasks", result.tasks_created.len()))
        } else {
            Err(AgentError::TaskExecutionFailed("Failed to create project plan".to_string()))
        }
    }
    
    /// Break down a large task into subtasks
    pub async fn breakdown_task(&self, task_id: &str) -> Result<Vec<String>, AgentError> {
        let result = self.planner_agent.breakdown_task(task_id).await?;
        
        if result.success {
            Ok(result.tasks_created)
        } else {
            Err(AgentError::TaskExecutionFailed("Failed to breakdown task".to_string()))
        }
    }
    
    /// Get task recommendations based on current context
    pub async fn get_task_recommendations(&self, context: &str, limit: usize) -> Result<Vec<String>, AgentError> {
        let suggestions = self.planner_agent.suggest_next_tasks(context, limit).await?;
        
        Ok(suggestions.into_iter()
            .map(|s| format!("{} (score: {:.2})", s.task.title, s.relevance_score))
            .collect())
    }
    
    /// Monitor agent activity across all tasks
    pub async fn get_agent_activity_dashboard(&self) -> Result<AgentActivityDashboard, AgentError> {
        let all_active_agents = self.task_manager.get_all_active_agents().await
            .map_err(|e| AgentError::DatabaseError(e.to_string()))?;
        
        let mut dashboard = AgentActivityDashboard {
            total_active_agents: 0,
            agents_by_task: all_active_agents.clone(),
            model_usage: std::collections::HashMap::new(),
            agent_performance: std::collections::HashMap::new(),
        };
        
        // Calculate statistics
        for (task_id, agents) in &all_active_agents {
            dashboard.total_active_agents += agents.len();
            
            for agent in agents {
                let model_key = format!("{}:{}", agent.model_info.provider, agent.model_info.model_name);
                *dashboard.model_usage.entry(model_key).or_insert(0) += 1;
                
                let performance = dashboard.agent_performance.entry(agent.agent_id.clone()).or_insert(AgentPerformanceMetrics {
                    tasks_assigned: 0,
                    tasks_completed: 0,
                    average_completion_time: 0.0,
                    success_rate: 0.0,
                });
                
                performance.tasks_assigned += 1;
                if agent.status == AgentTaskStatus::Completed {
                    performance.tasks_completed += 1;
                }
            }
        }
        
        // Calculate success rates
        for performance in dashboard.agent_performance.values_mut() {
            if performance.tasks_assigned > 0 {
                performance.success_rate = (performance.tasks_completed as f32 / performance.tasks_assigned as f32) * 100.0;
            }
        }
        
        Ok(dashboard)
    }
    
    // Helper methods
    
    async fn find_suitable_agents(&self, task: &crate::task_manager::Task, config: &TaskAssignmentConfig) -> Result<Vec<AgentInfo>, AgentError> {
        let mut suitable_agents = Vec::new();
        
        // Get available agents from registry
        let available_agents = self.agent_registry.list_agents().await;
        
        for agent_id in available_agents {
            if let Some(agent_type) = self.determine_agent_type_for_task(task) {
                if agent_id.contains(&agent_type.to_lowercase()) {
                    suitable_agents.push(AgentInfo {
                        agent_id: agent_id.clone(),
                        agent_type: agent_type.clone(),
                        model_info: ModelInfo {
                            provider: "OpenAI".to_string(), // Default - would be configurable
                            model_name: "gpt-4".to_string(),
                            version: Some("2024-01".to_string()),
                            context_window: Some(128000),
                            icon_url: Some("https://openai.com/favicon.ico".to_string()),
                        },
                        suitability_score: 0.8,
                    });
                }
            }
        }
        
        // Sort by suitability score
        suitable_agents.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap());
        
        Ok(suitable_agents)
    }
    
    fn determine_agent_type_for_task(&self, task: &crate::task_manager::Task) -> Option<String> {
        match task.task_type {
            crate::task_manager::TaskType::Feature => Some("Developer".to_string()),
            crate::task_manager::TaskType::Bug => Some("Debug".to_string()),
            crate::task_manager::TaskType::Testing => Some("Tester".to_string()),
            crate::task_manager::TaskType::Security => Some("Security".to_string()),
            crate::task_manager::TaskType::Performance => Some("Performance".to_string()),
            crate::task_manager::TaskType::Documentation => Some("Developer".to_string()),
            crate::task_manager::TaskType::Refactor => Some("Developer".to_string()),
            crate::task_manager::TaskType::Planning => Some("Architect".to_string()),
            _ => Some("Developer".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
struct AgentInfo {
    agent_id: String,
    agent_type: String,
    model_info: ModelInfo,
    suitability_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentActivityDashboard {
    pub total_active_agents: usize,
    pub agents_by_task: std::collections::HashMap<String, Vec<crate::task_manager::AgentAssignment>>,
    pub model_usage: std::collections::HashMap<String, usize>,
    pub agent_performance: std::collections::HashMap<String, AgentPerformanceMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerformanceMetrics {
    pub tasks_assigned: usize,
    pub tasks_completed: usize,
    pub average_completion_time: f32,
    pub success_rate: f32,
}
