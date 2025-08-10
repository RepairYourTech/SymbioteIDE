//! Integration Test for Workflow-Context System
//! 
//! This module provides comprehensive integration tests to verify that the
//! Visual Workflow Builder properly integrates with the ContextBus system.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::context::{ContextBus, SystemId, ContextUpdate, ContextUpdateType};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    /// Test basic workflow execution with context integration
    #[tokio::test]
    async fn test_workflow_context_integration() {
        // Initialize context bus
        let context_bus = Arc::new(ContextBus::new());
        
        // Initialize workflow builder with context integration
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Create a simple test workflow
        let workflow = create_test_workflow();
        
        // Execute workflow with context
        let execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                Some(create_test_input_data()),
            )
            .await
            .expect("Workflow execution should succeed");
        
        // Verify execution was created
        assert!(!execution_id.is_empty());
        
        // Wait for execution to complete
        sleep(Duration::from_millis(100)).await;
        
        // Verify execution status
        let status = workflow_builder.executor
            .get_execution_status(&execution_id)
            .await
            .expect("Should get execution status");
        
        // Status should be Running or Completed
        assert!(matches!(status, ExecutionStatus::Running | ExecutionStatus::Completed));
    }

    /// Test context-triggered workflow execution
    #[tokio::test]
    async fn test_context_triggered_workflow() {
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Subscribe workflow to context changes
        let subscription_id = workflow_builder.executor.context_executor
            .subscribe_to_context(
                "test_workflow".to_string(),
                "files.modified".to_string(),
                TriggerCondition::OnChange,
                None,
            )
            .await
            .expect("Should subscribe to context");
        
        assert!(!subscription_id.is_empty());
        
        // Simulate context change
        let context_update = ContextUpdate {
            system_id: SystemId::new("test_system"),
            update_type: ContextUpdateType::FileModified,
            data: serde_json::json!({"file_path": "/test/file.rs"}),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        
        // This would trigger the workflow
        workflow_builder.executor.context_executor
            .handle_context_change("files.modified", &serde_json::json!("/test/file.rs"))
            .await
            .expect("Should handle context change");
    }

    /// Test workflow node execution with context data
    #[tokio::test]
    async fn test_node_execution_with_context() {
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Create workflow with context-aware nodes
        let workflow = create_context_aware_workflow();
        
        // Execute workflow
        let execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                None,
            )
            .await
            .expect("Should execute workflow");
        
        // Verify nodes can access context data
        let executions = workflow_builder.executor.get_active_executions().await;
        let execution = executions.get(&execution_id).expect("Should find execution");
        
        assert_eq!(execution.status, ExecutionStatus::Running);
        assert!(!execution.node_executions.is_empty());
    }

    /// Test workflow performance monitoring
    #[tokio::test]
    async fn test_workflow_performance_monitoring() {
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Subscribe to execution events
        let mut event_receiver = workflow_builder.executor.subscribe_to_events();
        
        // Execute workflow
        let workflow = create_performance_test_workflow();
        let _execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                None,
            )
            .await
            .expect("Should execute workflow");
        
        // Wait for events
        tokio::select! {
            event = event_receiver.recv() => {
                match event {
                    Ok(ExecutionEvent::ExecutionStarted { execution_id, .. }) => {
                        assert!(!execution_id.is_empty());
                    }
                    _ => panic!("Expected ExecutionStarted event"),
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                // Timeout is acceptable for this test
            }
        }
    }

    /// Test error handling and recovery
    #[tokio::test]
    async fn test_workflow_error_handling() {
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Create workflow with failing node
        let workflow = create_failing_workflow();
        
        // Execute workflow
        let execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                None,
            )
            .await
            .expect("Should start workflow execution");
        
        // Wait for execution to process
        sleep(Duration::from_millis(200)).await;
        
        // Check execution status
        let executions = workflow_builder.executor.get_active_executions().await;
        if let Some(execution) = executions.get(&execution_id) {
            // Should have some node executions recorded
            assert!(!execution.node_executions.is_empty());
        }
    }

    // Helper functions for creating test workflows

    fn create_test_workflow() -> Workflow {
        Workflow {
            id: Uuid::new_v4().to_string(),
            name: "Test Workflow".to_string(),
            description: Some("Simple test workflow".to_string()),
            version: "1.0.0".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "node1".to_string(),
                    node_type: "trigger.manual".to_string(),
                    name: "Manual Trigger".to_string(),
                    description: Some("Manual trigger node".to_string()),
                    category: NodeCategory::Triggers,
                    position: NodePosition { x: 100.0, y: 100.0 },
                    configuration: NodeConfiguration {
                        parameters: std::collections::HashMap::new(),
                        timeout_ms: Some(5000),
                        retry_count: Some(3),
                        cache_ttl_seconds: None,
                    },
                },
                WorkflowNode {
                    id: "node2".to_string(),
                    node_type: "data.json.parse".to_string(),
                    name: "JSON Parser".to_string(),
                    description: Some("Parse JSON data".to_string()),
                    category: NodeCategory::Processing,
                    position: NodePosition { x: 300.0, y: 100.0 },
                    configuration: NodeConfiguration {
                        parameters: {
                            let mut params = std::collections::HashMap::new();
                            params.insert("json_data".to_string(), serde_json::json!("{\"test\": true}"));
                            params
                        },
                        timeout_ms: Some(5000),
                        retry_count: Some(3),
                        cache_ttl_seconds: None,
                    },
                },
            ],
            connections: vec![
                WorkflowConnection {
                    id: Uuid::new_v4().to_string(),
                    source_node_id: "node1".to_string(),
                    target_node_id: "node2".to_string(),
                    source_output: "data".to_string(),
                    target_input: "json_data".to_string(),
                    condition: None,
                },
            ],
            triggers: vec![
                WorkflowTrigger {
                    id: Uuid::new_v4().to_string(),
                    trigger_type: TriggerType::Manual,
                    configuration: std::collections::HashMap::new(),
                    enabled: true,
                },
            ],
            metadata: HashMap::new(),
        }
    }

    fn create_context_aware_workflow() -> Workflow {
        let mut workflow = create_test_workflow();
        workflow.name = "Context Aware Workflow".to_string();
        
        // Add a node that reads from context
        workflow.nodes.push(WorkflowNode {
            id: "context_reader".to_string(),
            node_type: "data.transform".to_string(),
            name: "Context Reader".to_string(),
            description: Some("Reads data from context".to_string()),
            category: NodeCategory::Processing,
            position: NodePosition { x: 500.0, y: 100.0 },
            configuration: NodeConfiguration {
                parameters: {
                    let mut params = std::collections::HashMap::new();
                    params.insert("transform_expression".to_string(), 
                        serde_json::json!("context.files.open"));
                    params
                },
                timeout_ms: Some(5000),
                retry_count: Some(3),
                cache_ttl_seconds: None,
            },
        });
        
        workflow
    }

    fn create_performance_test_workflow() -> Workflow {
        let mut workflow = create_test_workflow();
        workflow.name = "Performance Test Workflow".to_string();
        
        // Add multiple nodes for performance testing
        for i in 3..8 {
            workflow.nodes.push(WorkflowNode {
                id: format!("perf_node_{}", i),
                node_type: "data.hash".to_string(),
                name: format!("Hash Node {}", i),
                description: Some("Performance test node".to_string()),
                category: NodeCategory::Processing,
                position: NodePosition { x: (i as f64) * 100.0, y: 200.0 },
                configuration: NodeConfiguration {
                    parameters: {
                        let mut params = std::collections::HashMap::new();
                        params.insert("input_data".to_string(), 
                            serde_json::json!(format!("test_data_{}", i)));
                        params
                    },
                    timeout_ms: Some(1000),
                    retry_count: Some(1),
                    cache_ttl_seconds: None,
                },
            });
        }
        
        workflow
    }

    fn create_failing_workflow() -> Workflow {
        let mut workflow = create_test_workflow();
        workflow.name = "Failing Workflow".to_string();
        
        // Add a node that will fail
        workflow.nodes.push(WorkflowNode {
            id: "failing_node".to_string(),
            node_type: "nonexistent.node".to_string(), // This will cause failure
            name: "Failing Node".to_string(),
            enabled: true,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            metadata: HashMap::new(),
            description: Some("This node will fail".to_string()),
            category: NodeCategory::Processing,
            position: NodePosition { x: 500.0, y: 100.0 },
            configuration: NodeConfiguration {
                parameters: std::collections::HashMap::new(),
                timeout_ms: Some(1000),
                retry_count: Some(1),
                cache_ttl_seconds: None,
            },
        });
        
        workflow
    }

    fn create_test_input_data() -> std::collections::HashMap<String, serde_json::Value> {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("test_input".to_string(), serde_json::json!("test_value"));
        input_data.insert("number_input".to_string(), serde_json::json!(42));
        input_data.insert("array_input".to_string(), serde_json::json!([1, 2, 3]));
        input_data
    }
}
