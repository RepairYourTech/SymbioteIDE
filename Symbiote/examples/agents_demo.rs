//! # Symbiote Agents System Demo
//! 
//! This example demonstrates how to use the Symbiote multi-agent system
//! for intelligent development tasks.

use symbiote_core::{
    SymbioteAgentFramework, HiveMind, Symbiote, TeamManager, AgentBuilder,
    Result, UserId, ProjectId, AppConfig
};
use symbiote_core::agents::{
    TaskRequest, TaskType, TaskComplexity, TaskPriority,
    SymbioteDefinition, SymbioteSkill, SymbioteCapability, DevelopmentStack,
    SymbioteConfiguration, AllowedOperation, ResourceLimits, IsolationLevel,
    TaskConstraints, ParallelTaskRequest, IsolationRequirements
};
use symbiote_core::context::ContextBus;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    println!("🤖 Symbiote Agents System Demo");
    println!("===============================");

    // Initialize the context bus
    let context_bus = Arc::new(RwLock::new(ContextBus::new()));

    // Create the agent framework
    let framework = SymbioteAgentFramework::new(context_bus);

    // Initialize the framework
    framework.initialize().await?;
    println!("✅ Agent framework initialized");

    // Demo 1: Create a custom Symbiote
    println!("\n📝 Demo 1: Creating a Custom Symbiote");
    let user_id = UserId::new();
    let rust_specialist = create_rust_specialist(&framework, user_id.clone()).await?;
    println!("✅ Created Rust specialist: {}", rust_specialist);

    // Demo 2: Execute a simple task
    println!("\n🔧 Demo 2: Executing a Simple Task");
    let task_result = execute_simple_task(&framework, user_id.clone()).await?;
    println!("✅ Task completed: {:?}", task_result.status);

    // Demo 3: Team-based development
    println!("\n👥 Demo 3: Team-based Development");
    let team_result = execute_team_task(&framework, user_id.clone()).await?;
    println!("✅ Team task completed: {:?}", team_result.status);

    // Demo 4: Parallel task execution
    println!("\n⚡ Demo 4: Parallel Task Execution");
    let parallel_results = execute_parallel_tasks(&framework, user_id.clone()).await?;
    println!("✅ Parallel tasks completed: {} tasks", parallel_results.len());

    // Demo 5: Agent builder usage
    println!("\n🏗️ Demo 5: Visual Agent Builder");
    let custom_agent = build_custom_agent(&framework, user_id.clone()).await?;
    println!("✅ Custom agent built: {}", custom_agent);

    // Demo 6: Performance metrics
    println!("\n📊 Demo 6: Performance Metrics");
    let metrics = framework.get_performance_metrics().await?;
    println!("✅ Framework metrics: {} tasks completed", metrics.total_tasks_completed);

    println!("\n🎉 Demo completed successfully!");
    Ok(())
}

/// Create a specialized Rust development Symbiote
async fn create_rust_specialist(
    framework: &SymbioteAgentFramework,
    user_id: UserId,
) -> Result<String> {
    let definition = SymbioteDefinition {
        name: "Rust Development Specialist".to_string(),
        description: "Expert in Rust programming, testing, and optimization".to_string(),
        version: "1.0.0".to_string(),
        created_by: user_id,
        skills: vec![
            SymbioteSkill::Programming,
            SymbioteSkill::CodeGeneration,
            SymbioteSkill::CodeAnalysis,
            SymbioteSkill::TestGeneration,
            SymbioteSkill::Debugging,
            SymbioteSkill::StackSpecific(DevelopmentStack::Rust),
        ],
        capabilities: vec![
            SymbioteCapability::FileRead,
            SymbioteCapability::FileWrite,
            SymbioteCapability::CommandExecution,
            SymbioteCapability::GitOperations,
            SymbioteCapability::PackageManagement,
        ],
        specializations: vec![DevelopmentStack::Rust],
        configuration: SymbioteConfiguration {
            max_concurrent_tasks: 5,
            timeout_seconds: 300,
            memory_limit_mb: 1024,
            allowed_operations: vec![
                AllowedOperation::FileRead,
                AllowedOperation::FileWrite,
                AllowedOperation::CommandExecution,
                AllowedOperation::NetworkAccess,
            ],
            custom_settings: HashMap::new(),
        },
        metadata: HashMap::new(),
    };

    let mut builder = framework.agent_builder.write().await;
    let symbiote_id = builder.create_symbiote(definition, user_id).await?;
    
    Ok(symbiote_id)
}

/// Execute a simple development task
async fn execute_simple_task(
    framework: &SymbioteAgentFramework,
    user_id: UserId,
) -> Result<symbiote_core::agents::TaskExecution> {
    let task = TaskRequest {
        id: "simple-task-001".to_string(),
        task_type: TaskType::Development,
        description: "Create a simple Rust function to calculate fibonacci numbers".to_string(),
        complexity: TaskComplexity::Simple,
        priority: TaskPriority::Medium,
        user_id,
        project_id: Some(ProjectId::new()),
        context: HashMap::new(),
        constraints: TaskConstraints {
            timeout_seconds: Some(300),
            resource_limits: ResourceLimits {
                max_memory_mb: 512,
                max_cpu_cores: 2,
                max_disk_mb: 100,
                max_network_connections: 10,
            },
            allowed_operations: vec![
                AllowedOperation::FileRead,
                AllowedOperation::FileWrite,
                AllowedOperation::CommandExecution,
            ],
            isolation_level: IsolationLevel::Process,
        },
        metadata: HashMap::new(),
    };

    framework.execute_task(task).await
}

/// Execute a team-based development task
async fn execute_team_task(
    framework: &SymbioteAgentFramework,
    user_id: UserId,
) -> Result<symbiote_core::agents::TaskExecution> {
    let task = TaskRequest {
        id: "team-task-001".to_string(),
        task_type: TaskType::Development,
        description: "Build a complete web application with Rust backend and React frontend".to_string(),
        complexity: TaskComplexity::Complex,
        priority: TaskPriority::High,
        user_id,
        project_id: Some(ProjectId::new()),
        context: HashMap::new(),
        constraints: TaskConstraints {
            timeout_seconds: Some(1800), // 30 minutes
            resource_limits: ResourceLimits {
                max_memory_mb: 2048,
                max_cpu_cores: 4,
                max_disk_mb: 1000,
                max_network_connections: 50,
            },
            allowed_operations: vec![
                AllowedOperation::FileRead,
                AllowedOperation::FileWrite,
                AllowedOperation::CommandExecution,
                AllowedOperation::NetworkAccess,
                AllowedOperation::PackageInstallation,
            ],
            isolation_level: IsolationLevel::Container,
        },
        metadata: HashMap::new(),
    };

    framework.execute_task(task).await
}

/// Execute multiple tasks in parallel
async fn execute_parallel_tasks(
    framework: &SymbioteAgentFramework,
    user_id: UserId,
) -> Result<Vec<symbiote_core::agents::TaskExecution>> {
    let tasks = vec![
        ParallelTaskRequest {
            task: TaskRequest {
                id: "parallel-task-001".to_string(),
                task_type: TaskType::Testing,
                description: "Run unit tests for the authentication module".to_string(),
                complexity: TaskComplexity::Simple,
                priority: TaskPriority::Medium,
                user_id: user_id.clone(),
                project_id: Some(ProjectId::new()),
                context: HashMap::new(),
                constraints: TaskConstraints {
                    timeout_seconds: Some(120),
                    resource_limits: ResourceLimits {
                        max_memory_mb: 256,
                        max_cpu_cores: 1,
                        max_disk_mb: 50,
                        max_network_connections: 5,
                    },
                    allowed_operations: vec![
                        AllowedOperation::FileRead,
                        AllowedOperation::CommandExecution,
                    ],
                    isolation_level: IsolationLevel::Process,
                },
                metadata: HashMap::new(),
            },
            isolation_requirements: IsolationRequirements {
                separate_workspace: true,
                separate_worktree: true,
                network_isolation: false,
                resource_isolation: true,
            },
        },
        ParallelTaskRequest {
            task: TaskRequest {
                id: "parallel-task-002".to_string(),
                task_type: TaskType::Analysis,
                description: "Analyze code quality and suggest improvements".to_string(),
                complexity: TaskComplexity::Moderate,
                priority: TaskPriority::Low,
                user_id: user_id.clone(),
                project_id: Some(ProjectId::new()),
                context: HashMap::new(),
                constraints: TaskConstraints {
                    timeout_seconds: Some(180),
                    resource_limits: ResourceLimits {
                        max_memory_mb: 512,
                        max_cpu_cores: 2,
                        max_disk_mb: 100,
                        max_network_connections: 10,
                    },
                    allowed_operations: vec![
                        AllowedOperation::FileRead,
                        AllowedOperation::CommandExecution,
                    ],
                    isolation_level: IsolationLevel::Process,
                },
                metadata: HashMap::new(),
            },
            isolation_requirements: IsolationRequirements {
                separate_workspace: true,
                separate_worktree: false,
                network_isolation: false,
                resource_isolation: true,
            },
        },
    ];

    framework.execute_parallel_tasks(tasks).await
}

/// Build a custom agent using the visual builder
async fn build_custom_agent(
    framework: &SymbioteAgentFramework,
    user_id: UserId,
) -> Result<String> {
    // Get available templates
    let builder = framework.agent_builder.read().await;
    let templates = builder.get_available_templates().await?;
    
    if let Some(template) = templates.first() {
        println!("📋 Using template: {}", template.name);
        
        // Customize the template
        let customizations = symbiote_core::agents::TemplateCustomizations {
            name: Some("My Custom Developer Agent".to_string()),
            description: Some("A personalized development assistant".to_string()),
            additional_skills: Some(vec![
                SymbioteSkill::Documentation,
                SymbioteSkill::CodeReview,
            ]),
            additional_capabilities: Some(vec![
                SymbioteCapability::DatabaseAccess,
                SymbioteCapability::APIIntegration,
            ]),
            configuration_overrides: Some({
                let mut overrides = HashMap::new();
                overrides.insert("max_concurrent_tasks".to_string(), serde_json::Value::Number(10.into()));
                overrides.insert("enable_learning".to_string(), serde_json::Value::Bool(true));
                overrides
            }),
        };

        drop(builder);
        let mut builder = framework.agent_builder.write().await;
        let agent_id = builder.create_from_template(&template.id, customizations, user_id).await?;
        
        Ok(agent_id)
    } else {
        Err(symbiote_core::SymbioteError::context("No templates available".to_string()))
    }
}
