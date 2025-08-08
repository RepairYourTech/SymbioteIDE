// Tauri Commands for PA System Integration
use tauri::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use anyhow::Result;

use crate::pa_system::{
    PAIntegration, PASystemConfig, PASystemEvent, PlanGenerationRequest,
    GeneratedPlan, TaskAssignment, PASystemError
};

// Command payloads
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializePASystemRequest {
    pub config: PASystemConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratePlanRequest {
    pub request: PlanGenerationRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutePlanRequest {
    pub plan_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTasksRequest {
    pub plan_id: Option<String>,
    pub agent_id: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub task_id: String,
    pub status: Option<String>,
    pub progress: Option<f32>,
    pub notes: Option<String>,
}

// Tauri command handlers
#[tauri::command]
pub async fn initialize_pa_system(
    request: InitializePASystemRequest,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<bool, String> {
    let mut integration_guard = pa_integration.lock().await;
    
    match PAIntegration::new(
        request.config,
        Arc::new(RwLock::new(crate::task_manager::TaskManager::new())),
        Arc::new(crate::agent_orchestrator::AgentOrchestrator::new(
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(Vec::new())),
            Arc::new(RwLock::new(HashMap::new())),
            Arc::new(RwLock::new(HashMap::new()))
        ))
    ).await {
        Ok(integration) => {
            *integration_guard = Some(integration);
            Ok(true)
        }
        Err(e) => Err(format!("Failed to initialize PA system: {}", e))
    }
}

#[tauri::command]
pub async fn get_pa_status(
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<serde_json::Value, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.get_system_status().await {
            Ok(status) => Ok(serde_json::to_value(status).unwrap()),
            Err(e) => Err(format!("Failed to get PA status: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn generate_plan(
    request: GeneratePlanRequest,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<GeneratedPlan, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.generate_plan(request.request).await {
            Ok(plan) => Ok(plan),
            Err(e) => Err(format!("Failed to generate plan: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn execute_plan(
    request: ExecutePlanRequest,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<bool, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.execute_plan(&request.plan_id).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to execute plan: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_active_plans(
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<Vec<GeneratedPlan>, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.get_active_plans().await {
            Ok(plans) => Ok(plans),
            Err(e) => Err(format!("Failed to get active plans: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_tasks(
    request: GetTasksRequest,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<Vec<serde_json::Value>, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.get_tasks(request.plan_id, request.agent_id, request.status).await {
            Ok(tasks) => {
                let tasks_json: Vec<serde_json::Value> = tasks
                    .into_iter()
                    .map(|task| serde_json::to_value(task).unwrap())
                    .collect();
                Ok(tasks_json)
            }
            Err(e) => Err(format!("Failed to get tasks: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn update_task(
    request: UpdateTaskRequest,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<bool, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.update_task(
            &request.task_id,
            request.status,
            request.progress,
            request.notes
        ).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to update task: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_pa_agent_status(
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<Vec<serde_json::Value>, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.get_agent_status().await {
            Ok(agents) => {
                let agents_json: Vec<serde_json::Value> = agents
                    .into_iter()
                    .map(|agent| serde_json::to_value(agent).unwrap())
                    .collect();
                Ok(agents_json)
            }
            Err(e) => Err(format!("Failed to get agent status: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn assign_task_to_agent(
    task_id: String,
    agent_id: String,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<TaskAssignment, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.assign_task(&task_id, &agent_id).await {
            Ok(assignment) => Ok(assignment),
            Err(e) => Err(format!("Failed to assign task: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn start_websocket_server(
    port: u16,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<bool, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.start_websocket_server(port).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to start WebSocket server: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

#[tauri::command]
pub async fn get_recent_events(
    limit: Option<usize>,
    pa_integration: State<'_, Arc<Mutex<Option<PAIntegration>>>>
) -> Result<Vec<PASystemEvent>, String> {
    let integration_guard = pa_integration.lock().await;
    
    if let Some(integration) = integration_guard.as_ref() {
        match integration.get_recent_events(limit.unwrap_or(50)).await {
            Ok(events) => Ok(events),
            Err(e) => Err(format!("Failed to get recent events: {}", e))
        }
    } else {
        Err("PA system not initialized".to_string())
    }
}

// Export all commands for registration
// Note: This function is disabled due to Tauri API changes
/*
pub fn get_pa_system_commands() -> Vec<Box<dyn Fn(tauri::Invoke) + Send + Sync>> {
    vec![
        Box::new(tauri::generate_handler![
            initialize_pa_system,
            get_pa_status,
            generate_plan,
            execute_plan,
            get_active_plans,
            get_tasks,
            update_task,
            get_pa_agent_status,
            assign_task_to_agent,
            start_websocket_server,
            get_recent_events
        ])
    ]
}
*/
