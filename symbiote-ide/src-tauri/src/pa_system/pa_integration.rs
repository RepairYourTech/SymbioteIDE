// PA System Integration - Connects PA agent with existing SymbioteIDE systems
use crate::pa_system::*;
use crate::agents::agent_orchestrator::*;
use crate::task_manager::{TaskManager, Task, Project, Epic};
use crate::task_manager::manager::*;
use crate::symbiote_core::hybrid_intelligence::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::agent_runtime::AgentType;

/// PA System Integration Manager
pub struct PAIntegration {
    pub pa_agent: Arc<PAAgent>,
    pub task_manager: Arc<RwLock<TaskManager>>,
    pub agent_orchestrator: Arc<AgentOrchestrator>,
    pub hybrid_intelligence: Arc<HybridIntelligenceOrchestrator>,
    pub websocket_handler: Arc<RwLock<PAWebSocketHandler>>,
}

impl PAIntegration {
    pub async fn new(
        task_manager: Arc<RwLock<TaskManager>>,
        agent_orchestrator: Arc<AgentOrchestrator>,
        hybrid_intelligence: Arc<HybridIntelligenceOrchestrator>,
    ) -> Result<Self> {
        let config = PASystemConfig::default();
        
        // Create PA agent
        let pa_agent = Arc::new(PAAgent::new(
            config.clone(),
            Arc::clone(&task_manager),
            Arc::clone(&agent_orchestrator),
        ).await?);
        
        // Create WebSocket handler
        let websocket_handler = Arc::new(RwLock::new(PAWebSocketHandler::new(
            config,
            pa_agent.event_receiver.resubscribe(),
        )));
        
        Ok(Self {
            pa_agent,
            task_manager,
            agent_orchestrator,
            hybrid_intelligence,
            websocket_handler,
        })
    }
    
    /// Initialize PA system and start all services
    pub async fn initialize(&self) -> Result<()> {
        // Register PA agent with orchestrator
        self.agent_orchestrator.register_agent(
            self.pa_agent.agent_id.clone(),

        ).await?;
        
        // Start WebSocket server for real-time updates
        let mut websocket_handler = self.websocket_handler.write().await;
        tokio::spawn(async move {
            if let Err(e) = websocket_handler.start_server().await {
                eprintln!("Failed to start PA WebSocket server: {}", e);
            }
        });
        
        // Initialize PA agent with current project context
        self.initialize_pa_context().await?;
        
        println!("PA System initialized successfully!");
        Ok(())
    }
    
    /// Initialize PA agent with current project context
    async fn initialize_pa_context(&self) -> Result<()> {
        // Get current project information from hybrid intelligence
        let project_context = self.hybrid_intelligence.get_project_context().await?;
        
        // Update PA agent's planning context
        self.pa_agent.planning_context.current_project = Some(project_context.project_name);
        self.pa_agent.planning_context.available_resources = project_context.available_tools;
        self.pa_agent.planning_context.constraints = project_context.constraints;
        
        Ok(())
    }
    
    /// Handle user request for task planning
    pub async fn handle_planning_request(&self, request: PlanGenerationRequest) -> Result<GeneratedPlan> {
        // Use hybrid intelligence to enhance the request with context
        let enhanced_request = self.enhance_request_with_context(request).await?;
        
        // Generate plan using PA agent
        let plan = self.pa_agent.generate_plan(enhanced_request).await?;
        
        // Store plan in task manager
        self.store_plan_in_task_manager(&plan).await?;
        
        // Start coordinating task execution
        self.pa_agent.coordinate_task_execution(&plan).await?;
        
        Ok(plan)
    }
    
    /// Enhance planning request with codebase intelligence
    async fn enhance_request_with_context(&self, mut request: PlanGenerationRequest) -> Result<PlanGenerationRequest> {
        // Get relevant codebase context
        let codebase_context = self.hybrid_intelligence.get_relevant_context(
            &request.description,
            1000, // max tokens
        ).await?;
        
        // Enhance request with context
        if let Some(existing_context) = request.context {
            request.context = Some(format!("{}\n\nCodebase Context:\n{}", existing_context, codebase_context));
        } else {
            request.context = Some(format!("Codebase Context:\n{}", codebase_context));
        }
        
        Ok(request)
    }
    
    /// Store generated plan in task manager
    async fn store_plan_in_task_manager(&self, plan: &GeneratedPlan) -> Result<()> {
        let mut task_manager = self.task_manager.write().await;
        
        // Create project for the plan if needed
        let project_id = format!("plan-{}", plan.plan_id);
        let project = Project {
            id: project_id.clone(),
            name: plan.title.clone(),
            description: Some(plan.description.clone()),
            status: crate::task_manager::core::ProjectStatus::Active,
            created_at: plan.created_at,
            updated_at: plan.created_at,
            due_date: None,
            start_date: Some(plan.created_at),
            owner: "PA Agent".to_string(),
            team_members: vec!["PA Agent".to_string()],
            tags: vec!["ai-generated".to_string(), "plan".to_string()],
            repository_url: None,
            documentation_url: None,
            metadata: std::collections::HashMap::new(),
        };
        
        task_manager.create_project(project).await?;
        
        // Create tasks from the plan
        for task in &plan.tasks {
            task_manager.create_task(task.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Get real-time PA system status
    pub async fn get_pa_status(&self) -> Result<PASystemStatus> {
        let active_plans = self.pa_agent.active_plans.read().await;
        let available_agents = self.agent_orchestrator.get_available_agents().await?;
        
        Ok(PASystemStatus {
            pa_agent_id: self.pa_agent.agent_id.to_string(),
            active_plans_count: active_plans.len(),
            available_agents_count: available_agents.len(),
            websocket_clients_count: self.websocket_handler.read().await.clients.read().await.len(),
            last_activity: chrono::Utc::now(),
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PASystemStatus {
    pub pa_agent_id: String,
    pub active_plans_count: usize,
    pub available_agents_count: usize,
    pub websocket_clients_count: usize,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub project_name: String,
    pub available_tools: Vec<String>,
    pub constraints: Vec<String>,
    pub current_files: Vec<String>,
    pub recent_changes: Vec<String>,
}
