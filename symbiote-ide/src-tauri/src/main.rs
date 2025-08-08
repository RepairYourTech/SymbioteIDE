// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use tauri::{command, State, Manager};
mod task_manager;
// mod task_planner_agent; // Temporarily disabled
// mod task_manager_integration; // Temporarily disabled
mod interaction_modes;
mod pa_system;
mod performance_monitor;
mod api_builder;
mod symbiote_terminal;
mod context_manager;
mod codebase_intelligence;
mod agent_runtime;
mod agent_communication;
mod context_compression;
mod agent_orchestrator;
// mod enhanced_agent_runtime; // Temporarily disabled
// mod specialized_agents; // Temporarily disabled to fix core issues
mod qdrant_store;
mod neo4j_graph;
mod agents;
mod symbiote_core;
mod anti_duplication_impl;
mod anti_duplication_engine;
mod ai_parser;
mod ai_provider;
mod parser;

use context_manager::ContextManager;
use codebase_intelligence::CodebaseIntelligence;
use agent_runtime::{AgentRuntime, AgentType, AgentConfig, Permission, ResourceLimits};
use agent_communication::CommunicationProtocol;
use context_compression::{AutoContextCompressor, CompressionSettings};
use agent_orchestrator::{AgentOrchestrator, OrchestratedTask, TaskPriority, TaskStatus};
use task_manager::{TaskManager, TaskUpdateEvent, Project, ProjectStatus};
// Duplicate import removed - already imported above
// use task_planner_agent::TaskPlannerAgent; // Disabled
// use task_manager_integration::TaskManagerIntegration; // Disabled
use interaction_modes::ModeManager;
use parser::{ASTOptimization, LexicalEnhancement};
use pa_system::{PAIntegration, PASystemConfig};
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

// Stub types for missing modules
pub type AgentId = String;

#[derive(Debug, Serialize, Deserialize)]
struct AIRequest {
    provider: String,
    model: String,
    prompt: String,
    context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AIResponse {
    content: String,
    provider: String,
    model: String,
    tokens_used: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParseRequest {
    content: String,
    language: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ParseResponse {
    ast: String,
    symbols: Vec<String>,
    errors: Vec<String>,
}

// Core AI Provider Integration
#[tauri::command]
async fn call_ai_provider(request: AIRequest) -> Result<AIResponse, String> {
    // This will be implemented with our multi-provider orchestration
    ai_provider::call_provider(request).await
        .map_err(|e| e.to_string())
}

// Custom Parser Engine
#[tauri::command]
async fn parse_code(request: ParseRequest) -> Result<ParseResponse, String> {
    parser::parse_content(&request.content, &request.language)
        .map_err(|e| e.to_string())
}

// Agent System Commands
#[tauri::command]
async fn activate_agent(agent_type: String) -> Result<String, String> {
    // This will be connected to the global agent runtime
    Ok(format!("Agent {} activated with ID: {}", agent_type, "mock-id"))
}

#[tauri::command]
async fn execute_agent_task(task_json: String) -> Result<String, String> {
    // This will be connected to the global agent runtime
    Ok(format!("Task executed: {}", task_json))
}

#[tauri::command]
async fn get_agent_status(agent_id: String) -> Result<String, String> {
    // This will return actual agent health status
    Ok(format!("Agent {} status: Active", agent_id))
}

// Context Compression Commands
#[tauri::command]
async fn update_compression_settings(settings_json: String) -> Result<(), String> {
    // This will update the global context compressor settings
    Ok(())
}

#[tauri::command]
async fn get_compression_stats(agent_type: String) -> Result<String, String> {
    // This will return compression statistics for the agent type
    Ok(format!("Compression stats for {}: 0 compressions", agent_type))
}

// Anti-Duplication Engine Commands
#[tauri::command]
async fn analyze_duplicates(code: String, language: String) -> Result<String, String> {
    use anti_duplication_engine::AntiDuplicationEngine;
    
    let engine = AntiDuplicationEngine::new();
    match engine.analyze_code(&code, &language).await {
        Ok(analysis) => Ok(format!("Duplication analysis completed: {} patterns found", analysis.patterns.len())),
        Err(e) => Err(format!("Duplication analysis failed: {}", e))
    }
}

// Simple greeting command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to SymbioteIDE.", name)
}

// Context Management
#[tauri::command]
async fn get_context(scope: String, filters: Vec<String>) -> Result<String, String> {
    // This will be connected to the global context manager
    Ok(format!("Context for scope {}: mock data", scope))
}

#[tauri::command]
async fn update_context(scope: String, data: String) -> Result<(), String> {
    // This will update the global context manager
    Ok(())
}

// Interaction Mode Management Commands
#[tauri::command]
async fn switch_interaction_mode(
    mode: String,
) -> Result<String, String> {
    let interaction_mode = match mode.as_str() {
        "Easy" => interaction_modes::InteractionMode::Easy,
        "Interactive" => interaction_modes::InteractionMode::Interactive,
        "Manual" => interaction_modes::InteractionMode::Manual,
        _ => return Err("Invalid interaction mode".to_string()),
    };

    // For now, just return success - will be connected to global state later
    Ok(format!("Mode switched to {:?} successfully", interaction_mode))
}

#[tauri::command]
async fn get_current_mode() -> Result<String, String> {
    // For now, return Interactive as default - will be connected to global state later
    Ok("Interactive".to_string())
}

#[tauri::command]
async fn get_mode_config() -> Result<String, String> {
    // For now, return mock config - will be connected to global state later
    Ok("{\"autonomy_level\":\"Collaborative\",\"ui_complexity\":\"Collaborative\",\"agent_visibility\":\"Suggestions\"}".to_string())
}

// Additional commands for task management integration
#[tauri::command]
async fn ai_request(request: AIRequest) -> Result<AIResponse, String> {
    call_ai_provider(request).await
}

#[tauri::command]
async fn get_project_context(project_path: String) -> Result<String, String> {
    Ok(format!("Project context for: {}", project_path))
}

#[tauri::command]
async fn analyze_code(code: String, language: String) -> Result<String, String> {
    analyze_duplicates(code, language).await
}

#[tauri::command]
async fn get_suggestions(context: String) -> Result<String, String> {
    // This will integrate with the AI provider for contextual suggestions
    Ok(format!("Suggestions for context: {}", context))
}

// PA System Commands
#[tauri::command]
async fn pa_initialize_system(config: String) -> Result<String, String> {
    // Initialize PA system with configuration
    Ok(format!("PA System initialized with config: {}", config))
}

#[tauri::command]
async fn pa_generate_plan(request: String) -> Result<String, String> {
    // Generate AI-powered plan from request
    Ok(format!("Generated plan for request: {}", request))
}

#[tauri::command]
async fn pa_get_active_plans() -> Result<String, String> {
    // Get all active plans
    Ok("[]".to_string()) // Return empty array for now
}

#[tauri::command]
async fn pa_execute_plan(plan_id: String) -> Result<(), String> {
    // Execute a plan with given ID
    println!("Executing plan: {}", plan_id);
    Ok(())
}

#[tauri::command]
async fn pa_cancel_plan(plan_id: String) -> Result<(), String> {
    // Cancel a plan with given ID
    println!("Cancelling plan: {}", plan_id);
    Ok(())
}

#[tauri::command]
async fn pa_get_plan_progress(plan_id: String) -> Result<String, String> {
    // Get progress of a specific plan
    Ok(format!("{{\"plan_id\": \"{}\", \"progress\": 0, \"status\": \"pending\"}}", plan_id))
}

#[tauri::command]
async fn pa_create_task(task: String) -> Result<String, String> {
    // Create a new task
    Ok(format!("Created task: {}", task))
}

#[tauri::command]
async fn pa_update_task_status(task_id: String, status: String) -> Result<(), String> {
    // Update task status
    println!("Updated task {} to status: {}", task_id, status);
    Ok(())
}

#[tauri::command]
async fn pa_reassign_task(task_id: String, agent_id: String) -> Result<(), String> {
    // Reassign task to different agent
    println!("Reassigned task {} to agent: {}", task_id, agent_id);
    Ok(())
}

#[tauri::command]
async fn pa_get_system_status() -> Result<String, String> {
    // Get PA system status
    Ok("{\"status\": \"active\", \"agents_count\": 0, \"active_plans\": 0, \"websocket_clients\": 0}".to_string())
}

// API Builder Commands
#[tauri::command]
async fn api_builder_create_project(name: String, description: String) -> Result<String, String> {
    use api_builder::APIBuilder;
    
    let mut builder = APIBuilder::new();
    match builder.create_project(name, description) {
        Ok(project_id) => Ok(project_id),
        Err(e) => Err(format!("Failed to create project: {}", e))
    }
}

#[tauri::command]
async fn api_builder_get_project(project_id: String) -> Result<String, String> {
    use api_builder::APIBuilder;
    
    let builder = APIBuilder::new();
    match builder.get_project(&project_id) {
        Some(project) => {
            match serde_json::to_string(project) {
                Ok(json) => Ok(json),
                Err(e) => Err(format!("Failed to serialize project: {}", e))
            }
        },
        None => Err("Project not found".to_string())
    }
}

#[tauri::command]
async fn api_builder_add_endpoint(project_id: String, endpoint: String) -> Result<(), String> {
    use api_builder::{APIBuilder, APIEndpoint};
    
    let mut builder = APIBuilder::new();
    match serde_json::from_str::<APIEndpoint>(&endpoint) {
        Ok(endpoint_obj) => {
            match builder.add_endpoint(&project_id, endpoint_obj) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to add endpoint: {}", e))
            }
        },
        Err(e) => Err(format!("Invalid endpoint format: {}", e))
    }
}

#[tauri::command]
async fn api_builder_generate_code(project_id: String, framework: String) -> Result<String, String> {
    use api_builder::APIBuilder;
    
    let builder = APIBuilder::new();
    match framework.as_str() {
        "express" => {
            match builder.generate_express_code(&project_id) {
                Ok(code) => {
                    match serde_json::to_string(&code) {
                        Ok(json) => Ok(json),
                        Err(e) => Err(format!("Failed to serialize generated code: {}", e))
                    }
                },
                Err(e) => Err(format!("Failed to generate code: {}", e))
            }
        },
        _ => Err(format!("Unsupported framework: {}", framework))
    }
}

#[tauri::command]
async fn api_builder_get_templates() -> Result<String, String> {
    use api_builder::APIBuilder;
    
    let builder = APIBuilder::new();
    match serde_json::to_string(builder.get_templates()) {
        Ok(json) => Ok(json),
        Err(e) => Err(format!("Failed to serialize templates: {}", e))
    }
}

#[tauri::command]
async fn api_builder_apply_template(project_id: String, template_id: String) -> Result<(), String> {
    use api_builder::APIBuilder;
    
    let mut builder = APIBuilder::new();
    match builder.apply_template(&project_id, &template_id) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to apply template: {}", e))
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 SymbioteIDE starting with interaction modes support!");
    println!("📋 Available modes: Easy, Interactive, Manual");
    println!("🤖 Initializing multi-agent task management system...");
    
    // Initialize agent system
    if let Err(e) = initialize_agent_system().await {
        eprintln!("Failed to initialize agent system: {}", e);
        return;
    }
    
    println!("✅ Agent system initialized successfully!");
    println!("🎯 Starting SymbioteIDE with Day 1 interaction modes foundation...");
    
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(None::<PAIntegration>)))
        .invoke_handler(tauri::generate_handler![
            greet,
            call_ai_provider,
            parse_code,
            activate_agent,
            execute_agent_task,
            get_agent_status,
            update_compression_settings,
            get_compression_stats,
            analyze_duplicates,
            get_context,
            update_context,
            switch_interaction_mode,
            get_current_mode,
            get_mode_config,
            ai_request,
            get_project_context,
            analyze_code,
            get_suggestions,
            // PA System Commands
            pa_initialize_system,
            pa_generate_plan,
            pa_get_active_plans,
            pa_execute_plan,
            pa_cancel_plan,
            pa_get_plan_progress,
            pa_create_task,
            pa_update_task_status,
            pa_reassign_task,
            pa_get_system_status,
            // API Builder Commands
            api_builder_create_project,
            api_builder_get_project,
            api_builder_add_endpoint,
            api_builder_generate_code,
            api_builder_get_templates,
            api_builder_apply_template,
            // Legacy PA System Commands (from pa_system module)
            pa_system::initialize_pa_system,
            pa_system::get_pa_status,
            pa_system::generate_plan,
            pa_system::execute_plan,
            pa_system::get_active_plans,
            pa_system::get_tasks,
            pa_system::update_task,
            pa_system::get_pa_agent_status,
            pa_system::assign_task_to_agent,
            pa_system::start_websocket_server,
            pa_system::get_recent_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Initialize the agent system with default configurations
async fn initialize_agent_system() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing SymbioteIDE Agent System...");
    
    // Initialize core components
    let context_manager = ContextManager::new();
    let codebase_intelligence = CodebaseIntelligence::default();
    
    // Create agent runtime
    let agent_runtime = Arc::new(RwLock::new(None));
    let communication_protocol = Arc::new(RwLock::new(None));
    let context_compressor = Arc::new(RwLock::new(None));
    let agent_orchestrator = Arc::new(RwLock::new(None));
    let agent_context_integration = Arc::new(RwLock::new(None));
    let enhanced_agent_runtime = Arc::new(RwLock::new(None));
    let task_manager = Arc::new(RwLock::new(None));
    let task_planner_agent = Arc::new(RwLock::new(None));
    let task_integration = Arc::new(RwLock::new(None));
    
    // Initialize communication protocol
    *communication_protocol.write().unwrap() = Some(CommunicationProtocol::new());
    
    // Initialize context compressor
    *context_compressor.write().unwrap() = Some(AutoContextCompressor::new(CompressionSettings::default()));
    
    // Initialize agent runtime
    *agent_runtime.write().unwrap() = Some(AgentRuntime::new(context_manager, codebase_intelligence));
    
    // Initialize agent orchestrator
    *agent_orchestrator.write().unwrap() = Some(AgentOrchestrator::new(
        Arc::new(RwLock::new(HashMap::new())),
        Arc::new(RwLock::new(HashMap::new())),
        Arc::new(RwLock::new(Vec::new())),
        Arc::new(RwLock::new(HashMap::new())),
        Arc::new(RwLock::new(HashMap::new()))
    ));
    
    // Register default agent configurations
    if let Some(runtime) = agent_runtime.read().unwrap().as_ref() {
        register_default_agent_configs(runtime).await?;
    }
    
    println!("Agent system initialized successfully!");
    Ok(())
}

/// Register default configurations for all agent types
async fn register_default_agent_configs(runtime: &AgentRuntime) -> Result<(), Box<dyn std::error::Error>> {
    // Core agents
    runtime.register_agent_config(create_agent_config(
        AgentType::Orchestrator,
        "claude-3-sonnet-20241022".to_string(),
        1000000, // 1M tokens for orchestrator
        vec![Permission::ReadCode, Permission::WriteCode, Permission::ExecuteCommands],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::Architect,
        "claude-3-sonnet-20241022".to_string(),
        200000, // 200K tokens for architect
        vec![Permission::ReadCode, Permission::WriteCode],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::Developer,
        "claude-3-sonnet-20241022".to_string(),
        100000, // 100K tokens for developer
        vec![Permission::ReadCode, Permission::WriteCode, Permission::AccessFileSystem],
    )).await?;
    
    // Language specialists
    runtime.register_agent_config(create_agent_config(
        AgentType::RustSpecialist,
        "deepseek-coder".to_string(),
        32000, // 32K tokens for Rust specialist
        vec![Permission::ReadCode, Permission::WriteCode, Permission::AccessFileSystem],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::ReactSpecialist,
        "claude-3-sonnet-20241022".to_string(),
        24000, // 24K tokens for React specialist
        vec![Permission::ReadCode, Permission::WriteCode, Permission::AccessFileSystem],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::TypeScriptSpecialist,
        "claude-3-sonnet-20241022".to_string(),
        28000, // 28K tokens for TypeScript specialist
        vec![Permission::ReadCode, Permission::WriteCode, Permission::AccessFileSystem],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::Debug,
        "claude-3-sonnet-20241022".to_string(),
        16000, // 16K tokens for debug agent
        vec![Permission::ReadCode, Permission::AccessFileSystem, Permission::ExecuteCommands],
    )).await?;
    
    // Specialized agents
    runtime.register_agent_config(create_agent_config(
        AgentType::Test,
        "claude-3-sonnet-20241022".to_string(),
        20000, // 20K tokens for test agent
        vec![Permission::ReadCode, Permission::WriteCode, Permission::ExecuteCommands],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::Security,
        "claude-3-sonnet-20241022".to_string(),
        28000, // 28K tokens for security agent
        vec![Permission::ReadCode, Permission::AccessSecrets],
    )).await?;
    
    runtime.register_agent_config(create_agent_config(
        AgentType::Deployment,
        "claude-3-sonnet-20241022".to_string(),
        24000, // 24K tokens for deployment agent
        vec![Permission::ReadCode, Permission::DeployCode, Permission::ManageInfrastructure, Permission::NetworkAccess],
    )).await?;
    
    println!("Registered {} agent configurations", 10);
    Ok(())
}

/// Create a standard agent configuration
fn create_agent_config(
    agent_type: AgentType,
    model_name: String,
    max_tokens: u32,
    permissions: Vec<Permission>,
) -> AgentConfig {
    AgentConfig {
        agent_type,
        model_provider: "anthropic".to_string(), // Default to Anthropic
        model_name,
        max_tokens,
        temperature: 0.1, // Low temperature for consistent results
        timeout_seconds: 300, // 5 minute timeout
        retry_attempts: 3,
        permissions,
        resource_limits: ResourceLimits {
            max_memory_mb: 512,
            max_cpu_percent: 50,
            max_execution_time_seconds: 300,
            max_file_operations: 1000,
            max_network_requests: 100,
        },
    }
}
