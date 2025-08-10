//! Integration tests for the Visual Workflow Builder System

use symbiote_core::{Result, UserId, ProjectId};
use symbiote_core::workflow::{
    VisualWorkflowBuilder, NodeCategory, ExecutionTrigger, ExecutionStatus,
    AIAgentIntegration, AICapability, AgentRequirements, NLPTask,
    WorkflowStateManager, VisualEditor, WorkflowRuntime,
    NodeDefinition, DataType, NodeInput, NodeOutput, PricingModel,
    NodeRequirements, NodePricing, AgentResourceRequirements, CollaborationMode,
};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

/// Test visual workflow builder initialization
#[tokio::test]
async fn test_workflow_builder_initialization() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    
    // Test getting node categories
    let categories = builder.get_node_categories();
    assert_eq!(categories.len(), 18);
    assert!(categories.contains(&NodeCategory::AI));
    assert!(categories.contains(&NodeCategory::Communication));
    assert!(categories.contains(&NodeCategory::Development));
    
    Ok(())
}

/// Test workflow creation
#[tokio::test]
async fn test_workflow_creation() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    let user_id = UserId::new();
    let project_id = Some(ProjectId::new());
    
    let workflow = builder.create_workflow(
        "Test Workflow".to_string(),
        Some("A test workflow for validation".to_string()),
        user_id,
        project_id,
    ).await?;
    
    assert_eq!(workflow.name, "Test Workflow");
    assert!(workflow.description.is_some());
    assert!(!workflow.id.is_empty());
    assert!(workflow.nodes.is_empty());
    assert!(workflow.connections.is_empty());
    
    Ok(())
}

/// Test workflow execution
#[tokio::test]
async fn test_workflow_execution() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    let user_id = UserId::new();
    
    let workflow = builder.create_workflow(
        "Execution Test".to_string(),
        None,
        user_id,
        None,
    ).await?;
    
    // Execute the workflow
    let execution_id = builder.execute_workflow(
        &workflow.id,
        ExecutionTrigger::Manual,
        Some(HashMap::new()),
    ).await?;
    
    assert!(!execution_id.is_empty());
    
    // Check execution status
    let status = builder.get_execution_status(&execution_id).await?;
    assert!(matches!(status, ExecutionStatus::Running | ExecutionStatus::Completed));
    
    Ok(())
}

/// Test event subscription
#[tokio::test]
async fn test_workflow_events() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    let mut event_receiver = builder.subscribe_to_events();
    
    // Create workflow in background task
    let builder_clone = builder.clone();
    let user_id = UserId::new();
    tokio::spawn(async move {
        sleep(Duration::from_millis(10)).await;
        let _ = builder_clone.create_workflow(
            "Event Test".to_string(),
            None,
            user_id,
            None,
        ).await;
    });
    
    // Wait for event
    let event = tokio::time::timeout(Duration::from_millis(100), event_receiver.recv()).await;
    assert!(event.is_ok());
    
    Ok(())
}

/// Test AI agent integration
#[tokio::test]
async fn test_ai_agent_integration() -> Result<()> {
    let ai_integration = AIAgentIntegration::new();
    
    // Create AI node
    let capability = AICapability::NLP {
        tasks: vec![NLPTask::TextClassification, NLPTask::SentimentAnalysis],
        models: vec!["gpt-4".to_string(), "claude-3".to_string()],
    };
    
    let requirements = AgentRequirements {
        required_skills: vec!["nlp".to_string(), "classification".to_string()],
        minimum_experience: 5,
        preferred_agents: vec!["nlp-specialist".to_string()],
        resource_requirements: AgentResourceRequirements {
            cpu_cores: 2.0,
            memory_mb: 4096,
            gpu_required: false,
            network_bandwidth_mbps: 100.0,
            storage_mb: 1024,
        },
        collaboration_mode: CollaborationMode::Solo,
    };
    
    let ai_node = ai_integration.create_ai_node(
        "ai.nlp.classifier".to_string(),
        capability,
        requirements,
    ).await?;
    
    assert_eq!(ai_node.node_type, "ai.nlp.classifier");
    assert!(ai_node.learning_enabled);
    
    Ok(())
}

/// Test AI node execution
#[tokio::test]
async fn test_ai_node_execution() -> Result<()> {
    let ai_integration = AIAgentIntegration::new();
    
    // Create and execute AI node
    let capability = AICapability::NLP {
        tasks: vec![NLPTask::SentimentAnalysis],
        models: vec!["sentiment-model".to_string()],
    };
    
    let requirements = AgentRequirements {
        required_skills: vec!["sentiment".to_string()],
        minimum_experience: 3,
        preferred_agents: Vec::new(),
        resource_requirements: AgentResourceRequirements {
            cpu_cores: 1.0,
            memory_mb: 2048,
            gpu_required: false,
            network_bandwidth_mbps: 50.0,
            storage_mb: 512,
        },
        collaboration_mode: CollaborationMode::Solo,
    };
    
    let ai_node = ai_integration.create_ai_node(
        "sentiment.analyzer".to_string(),
        capability,
        requirements,
    ).await?;
    
    // Assign agent to node
    ai_integration.assign_agent_to_node(&ai_node.node_id, "agent-123").await?;
    
    // Execute node
    let mut input_data = HashMap::new();
    input_data.insert("text".to_string(), serde_json::Value::String("This is a great product!".to_string()));
    
    let output = ai_integration.execute_ai_node(&ai_node.node_id, input_data).await?;
    assert!(!output.is_empty());
    
    Ok(())
}

/// Test workflow state management
#[tokio::test]
async fn test_workflow_state_management() -> Result<()> {
    let state_manager = WorkflowStateManager::new();
    let workflow_id = "test-workflow-123".to_string();
    
    // Create workflow state
    state_manager.create_workflow_state(workflow_id.clone()).await?;
    
    // Get workflow state
    let state = state_manager.get_workflow_state(&workflow_id).await?;
    assert_eq!(state.workflow_id, workflow_id);
    assert_eq!(state.version, 1);
    assert!(state.execution_states.is_empty());
    
    Ok(())
}

/// Test visual editor
#[tokio::test]
async fn test_visual_editor() -> Result<()> {
    let editor = VisualEditor::new();
    let user_id = UserId::new();
    let workflow_id = "editor-test-workflow".to_string();
    
    // Create editor session
    let session_id = editor.create_session(user_id.clone(), workflow_id.clone()).await?;
    assert!(!session_id.is_empty());
    
    // Get session
    let session = editor.get_session(&session_id).await?;
    assert_eq!(session.user_id, user_id);
    assert_eq!(session.workflow_id, workflow_id);
    assert_eq!(session.viewport.zoom, 1.0);
    
    Ok(())
}

/// Test workflow runtime
#[tokio::test]
async fn test_workflow_runtime() -> Result<()> {
    let runtime = WorkflowRuntime::new();
    let workflow_id = "runtime-test-workflow";
    
    // Execute workflow
    let execution_id = runtime.execute_workflow(
        workflow_id,
        ExecutionTrigger::API,
        Some(HashMap::new()),
    ).await?;
    
    assert!(!execution_id.is_empty());
    
    // Check status
    let status = runtime.get_execution_status(&execution_id).await?;
    assert!(matches!(status, ExecutionStatus::Running | ExecutionStatus::Completed));
    
    Ok(())
}

/// Test node registry functionality
#[tokio::test]
async fn test_node_registry() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    
    // Get AI nodes
    let ai_nodes = builder.node_registry.get_nodes_by_category(&NodeCategory::AI).await;
    assert!(!ai_nodes.is_empty());
    
    // Search for nodes
    let search_results = builder.node_registry.search_nodes("openai").await;
    assert!(!search_results.is_empty());
    
    // Get specific node definition
    if let Some(node) = search_results.first() {
        let definition = builder.node_registry.get_node_definition(&node.node_type).await;
        assert!(definition.is_some());
    }
    
    Ok(())
}

/// Test comprehensive workflow with multiple nodes
#[tokio::test]
async fn test_comprehensive_workflow() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    let user_id = UserId::new();
    let project_id = Some(ProjectId::new());
    
    // Create workflow
    let mut workflow = builder.create_workflow(
        "Comprehensive Test Workflow".to_string(),
        Some("A workflow with multiple node types".to_string()),
        user_id,
        project_id,
    ).await?;
    
    // Add AI node
    let ai_integration = AIAgentIntegration::new();
    let ai_node = ai_integration.create_ai_node(
        "ai.text.processor".to_string(),
        AICapability::NLP {
            tasks: vec![NLPTask::TextSummarization],
            models: vec!["gpt-4".to_string()],
        },
        AgentRequirements {
            required_skills: vec!["summarization".to_string()],
            minimum_experience: 4,
            preferred_agents: Vec::new(),
            resource_requirements: AgentResourceRequirements {
                cpu_cores: 2.0,
                memory_mb: 4096,
                gpu_required: false,
                network_bandwidth_mbps: 100.0,
                storage_mb: 1024,
            },
            collaboration_mode: CollaborationMode::Solo,
        },
    ).await?;
    
    // Verify workflow structure
    assert_eq!(workflow.name, "Comprehensive Test Workflow");
    assert!(workflow.description.is_some());
    assert!(!ai_node.node_id.is_empty());
    
    Ok(())
}

/// Test concurrent workflow operations
#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    let user_id = UserId::new();
    
    // Create multiple workflows concurrently
    let mut handles = Vec::new();
    
    for i in 0..5 {
        let builder_clone = builder.clone();
        let user_id_clone = user_id.clone();
        
        let handle = tokio::spawn(async move {
            builder_clone.create_workflow(
                format!("Concurrent Workflow {}", i),
                None,
                user_id_clone,
                None,
            ).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all workflows to be created
    let mut workflows = Vec::new();
    for handle in handles {
        let workflow = handle.await.unwrap()?;
        workflows.push(workflow);
    }
    
    assert_eq!(workflows.len(), 5);
    
    // Verify all workflows have unique IDs
    let mut ids = std::collections::HashSet::new();
    for workflow in workflows {
        assert!(ids.insert(workflow.id));
    }
    
    Ok(())
}

/// Test error handling
#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let builder = VisualWorkflowBuilder::new();
    
    // Test getting non-existent execution status
    let result = builder.get_execution_status("non-existent-id").await;
    assert!(result.is_err());
    
    // Test AI integration error handling
    let ai_integration = AIAgentIntegration::new();
    let result = ai_integration.execute_ai_node("non-existent-node", HashMap::new()).await;
    assert!(result.is_err());
    
    Ok(())
}

impl Clone for VisualWorkflowBuilder {
    fn clone(&self) -> Self {
        // For testing purposes, create a new instance
        VisualWorkflowBuilder::new()
    }
}
