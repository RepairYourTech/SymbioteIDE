//! End-to-End Integration Test
//! 
//! This module provides comprehensive end-to-end tests that verify the complete
//! workflow system works together: ContextBus + WorkflowExecutor + NodeExecutionEngine

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::context::{ContextBus, SystemId, ContextUpdate, ContextUpdateType};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    use uuid::Uuid;

    /// Test complete end-to-end workflow execution with real node execution
    #[tokio::test]
    async fn test_end_to_end_workflow_execution() {
        // Initialize the complete system
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Create a realistic workflow with actual nodes
        let workflow = create_realistic_workflow();
        
        // Execute workflow end-to-end
        let execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                Some(create_realistic_input_data()),
            )
            .await
            .expect("End-to-end workflow execution should succeed");
        
        // Verify execution was created
        assert!(!execution_id.is_empty());
        
        // Wait for execution to process
        sleep(Duration::from_millis(500)).await;
        
        // Verify execution status
        let status = workflow_builder.executor
            .get_execution_status(&execution_id)
            .await
            .expect("Should get execution status");
        
        // Should be Running or Completed
        assert!(matches!(status, ExecutionStatus::Running | ExecutionStatus::Completed));
        
        // Verify nodes were actually executed
        let executions = workflow_builder.executor.get_active_executions().await;
        let execution = executions.get(&execution_id).expect("Should find execution");
        
        // Should have node executions recorded
        assert!(!execution.node_executions.is_empty());
        
        // Verify at least one node completed successfully
        let successful_nodes = execution.node_executions.values()
            .filter(|node_exec| matches!(node_exec.status, NodeExecutionStatus::Completed))
            .count();
        
        assert!(successful_nodes > 0, "At least one node should complete successfully");
    }

    /// Test context-triggered workflow with real node execution
    #[tokio::test]
    async fn test_context_triggered_workflow_execution() {
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
        
        // Simulate context change that should trigger workflow
        let context_update = ContextUpdate {
            system_id: SystemId::new("test_system"),
            update_type: ContextUpdateType::FileModified,
            data: serde_json::json!({"file_path": "/test/important_file.rs"}),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        };
        
        // Update context bus
        context_bus.update_context(context_update).await
            .expect("Should update context");
        
        // Trigger workflow execution based on context change
        workflow_builder.executor.context_executor
            .handle_context_change("files.modified", &serde_json::json!("/test/important_file.rs"))
            .await
            .expect("Should handle context change");
        
        // Verify subscription exists
        let subscriptions = workflow_builder.executor.context_executor
            .get_workflow_subscriptions("test_workflow")
            .await;
        
        assert!(!subscriptions.is_empty());
    }

    /// Test node execution engine with various node types
    #[tokio::test]
    async fn test_node_execution_engine_comprehensive() {
        let node_executor = NodeExecutionEngine::new();

        // Test different node categories
        let test_cases = vec![
            // Trigger node
            (create_trigger_node(), create_trigger_context()),
            // Communication node
            (create_communication_node(), create_communication_context()),
            // Processing node
            (create_processing_node(), create_processing_context()),
            // AI node
            (create_ai_node(), create_ai_context()),
            // Utility node
            (create_utility_node(), create_utility_context()),
        ];

        for (node_definition, context) in test_cases {
            let result = node_executor.execute_node(&node_definition, context).await
                .expect("Node execution should succeed");

            assert!(result.success, "Node execution should be successful");
            assert!(!result.output_data.is_empty(), "Node should produce output");
            assert!(result.execution_time_ms >= 0, "Should track execution time");
        }
    }

    /// Test OpenRouter and Gemini nodes specifically
    #[tokio::test]
    async fn test_openrouter_and_gemini_nodes() {
        let node_executor = NodeExecutionEngine::new();

        // Test OpenRouter node (will fail without real API key, but should validate inputs)
        let openrouter_node = create_openrouter_node();
        let openrouter_context = create_openrouter_context();

        let openrouter_result = node_executor.execute_node(&openrouter_node, openrouter_context).await;

        // Should fail due to missing/invalid API key, but should validate the structure
        match openrouter_result {
            Ok(_) => {
                // If it succeeds, great! (would need real API key)
                println!("OpenRouter node executed successfully");
            }
            Err(e) => {
                // Should fail with API error, not validation error
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("OpenRouter API") || error_msg.contains("API key"),
                    "Should fail with API error, not validation error: {}",
                    error_msg
                );
            }
        }

        // Test Gemini node (will fail without real API key, but should validate inputs)
        let gemini_node = create_gemini_node();
        let gemini_context = create_gemini_context();

        let gemini_result = node_executor.execute_node(&gemini_node, gemini_context).await;

        // Should fail due to missing/invalid API key, but should validate the structure
        match gemini_result {
            Ok(_) => {
                // If it succeeds, great! (would need real API key)
                println!("Gemini node executed successfully");
            }
            Err(e) => {
                // Should fail with API error, not validation error
                let error_msg = e.to_string();
                assert!(
                    error_msg.contains("Gemini API") || error_msg.contains("API key"),
                    "Should fail with API error, not validation error: {}",
                    error_msg
                );
            }
        }
    }

    /// Test Chat Trigger Node specifically
    #[tokio::test]
    async fn test_chat_trigger_node() {
        let node_executor = NodeExecutionEngine::new();

        // Test different types of chat messages
        let test_cases = vec![
            // Code generation intent
            ("Generate a Python function to calculate fibonacci numbers", "code_generation"),
            // Code analysis intent
            ("Analyze the performance of my Rust code in main.rs", "code_analysis"),
            // Documentation intent
            ("Create documentation for my API endpoints", "documentation_generation"),
            // Deployment intent
            ("Deploy my application to production", "deployment"),
            // Communication intent
            ("Send a message to the team about the release", "communication"),
            // General query
            ("What's the weather like?", "general_query"),
        ];

        for (message, expected_intent_type) in test_cases {
            let chat_node = create_chat_trigger_node();
            let chat_context = create_chat_trigger_context(message);

            let result = node_executor.execute_node(&chat_node, chat_context).await
                .expect("Chat trigger node should execute successfully");

            assert!(result.success, "Chat trigger should succeed");
            assert!(!result.output_data.is_empty(), "Should produce output");

            // Verify the output structure
            assert!(result.output_data.contains_key("triggered"), "Should have triggered flag");
            assert!(result.output_data.contains_key("user_message"), "Should have user message");
            assert!(result.output_data.contains_key("intent"), "Should have parsed intent");
            assert!(result.output_data.contains_key("entities"), "Should have extracted entities");
            assert!(result.output_data.contains_key("suggested_workflows"), "Should have suggested workflows");

            // Verify intent parsing
            if let Some(intent) = result.output_data.get("intent") {
                if let Some(intent_type) = intent.get("type").and_then(|t| t.as_str()) {
                    assert_eq!(intent_type, expected_intent_type,
                        "Intent should be correctly parsed for message: '{}'", message);
                }
            }

            // Verify triggered is true
            assert_eq!(result.output_data.get("triggered"), Some(&serde_json::json!(true)));

            println!("✅ Chat trigger test passed for: '{}'", message);
        }
    }

    /// Test Chat Trigger with file extraction
    #[tokio::test]
    async fn test_chat_trigger_file_extraction() {
        let node_executor = NodeExecutionEngine::new();

        let chat_node = create_chat_trigger_node();
        let message = "Please analyze the code in src/main.rs and tests/integration_test.py";
        let chat_context = create_chat_trigger_context(message);

        let result = node_executor.execute_node(&chat_node, chat_context).await
            .expect("Chat trigger should execute successfully");

        assert!(result.success, "Chat trigger should succeed");

        // Verify file extraction
        if let Some(entities) = result.output_data.get("entities") {
            if let Some(files) = entities.get("files").and_then(|f| f.as_array()) {
                assert!(!files.is_empty(), "Should extract file paths");

                let file_strings: Vec<String> = files.iter()
                    .filter_map(|f| f.as_str().map(String::from))
                    .collect();

                assert!(file_strings.contains(&"src/main.rs".to_string()), "Should extract main.rs");
                assert!(file_strings.contains(&"tests/integration_test.py".to_string()), "Should extract test file");
            }
        }

        println!("✅ Chat trigger file extraction test passed");
    }

    /// Test Chat Trigger with language detection
    #[tokio::test]
    async fn test_chat_trigger_language_detection() {
        let node_executor = NodeExecutionEngine::new();

        let chat_node = create_chat_trigger_node();
        let message = "Write a Rust function and a Python script for data processing";
        let chat_context = create_chat_trigger_context(message);

        let result = node_executor.execute_node(&chat_node, chat_context).await
            .expect("Chat trigger should execute successfully");

        assert!(result.success, "Chat trigger should succeed");

        // Verify language detection
        if let Some(entities) = result.output_data.get("entities") {
            if let Some(languages) = entities.get("languages").and_then(|l| l.as_array()) {
                let lang_strings: Vec<String> = languages.iter()
                    .filter_map(|l| l.as_str().map(String::from))
                    .collect();

                assert!(lang_strings.contains(&"rust".to_string()), "Should detect Rust");
                assert!(lang_strings.contains(&"python".to_string()), "Should detect Python");
            }
        }

        println!("✅ Chat trigger language detection test passed");
    }

    /// Test workflow with error handling and recovery
    #[tokio::test]
    async fn test_workflow_error_handling_and_recovery() {
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Create workflow with a node that will fail
        let workflow = create_workflow_with_failing_node();
        
        // Execute workflow
        let execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                None,
            )
            .await
            .expect("Should start workflow execution even with failing nodes");
        
        // Wait for execution to process
        sleep(Duration::from_millis(300)).await;
        
        // Check execution results
        let executions = workflow_builder.executor.get_active_executions().await;
        if let Some(execution) = executions.get(&execution_id) {
            // Should have attempted to execute nodes
            assert!(!execution.node_executions.is_empty());
            
            // Should have some failed nodes
            let failed_nodes = execution.node_executions.values()
                .filter(|node_exec| matches!(node_exec.status, NodeExecutionStatus::Failed))
                .count();
            
            // At least one node should fail (the intentionally failing one)
            assert!(failed_nodes > 0, "Should have failed nodes for error handling test");
        }
    }

    /// Test performance monitoring and metrics
    #[tokio::test]
    async fn test_workflow_performance_monitoring() {
        let context_bus = Arc::new(ContextBus::new());
        let workflow_builder = VisualWorkflowBuilder::new(Arc::clone(&context_bus));
        
        // Subscribe to execution events
        let mut event_receiver = workflow_builder.executor.subscribe_to_events();
        
        // Execute workflow
        let workflow = create_performance_monitoring_workflow();
        let execution_id = workflow_builder
            .execute_workflow_with_context(
                workflow,
                ExecutionTrigger::Manual,
                None,
            )
            .await
            .expect("Should execute workflow");
        
        // Collect events for a short time
        let mut events_received = 0;
        let timeout_duration = Duration::from_millis(200);
        
        tokio::select! {
            _ = async {
                while let Ok(event) = event_receiver.recv().await {
                    events_received += 1;
                    match event {
                        ExecutionEvent::ExecutionStarted { execution_id: exec_id, .. } => {
                            assert_eq!(exec_id, execution_id);
                        }
                        ExecutionEvent::NodeStarted { execution_id: exec_id, .. } => {
                            assert_eq!(exec_id, execution_id);
                        }
                        ExecutionEvent::NodeCompleted { execution_id: exec_id, duration_ms, .. } => {
                            assert_eq!(exec_id, execution_id);
                            assert!(duration_ms >= 0);
                        }
                        _ => {}
                    }
                    
                    // Stop after receiving a few events
                    if events_received >= 3 {
                        break;
                    }
                }
            } => {}
            _ = sleep(timeout_duration) => {}
        }
        
        // Should have received some events
        assert!(events_received > 0, "Should receive performance monitoring events");
    }

    // Helper functions for creating test data

    fn create_realistic_workflow() -> Workflow {
        Workflow {
            id: Uuid::new_v4().to_string(),
            name: "Realistic Test Workflow".to_string(),
            description: Some("End-to-end test workflow with real nodes".to_string()),
            version: "1.0.0".to_string(),
            nodes: vec![
                // Manual trigger
                WorkflowNode {
                    id: "trigger_node".to_string(),
                    node_type: "trigger.manual".to_string(),
                    name: "Manual Trigger".to_string(),
                    description: Some("Start the workflow manually".to_string()),
                    category: NodeCategory::Triggers,
                    position: NodePosition { x: 100.0, y: 100.0 },
                    configuration: NodeConfiguration {
                        parameters: std::collections::HashMap::new(),
                        timeout_ms: Some(5000),
                        retry_count: Some(1),
                        cache_ttl_seconds: None,
                    },
                },
                // JSON processing
                WorkflowNode {
                    id: "json_parser".to_string(),
                    node_type: "data.json.parse".to_string(),
                    name: "JSON Parser".to_string(),
                    description: Some("Parse JSON data".to_string()),
                    category: NodeCategory::Processing,
                    position: NodePosition { x: 300.0, y: 100.0 },
                    configuration: NodeConfiguration {
                        parameters: {
                            let mut params = std::collections::HashMap::new();
                            params.insert("json_data".to_string(), serde_json::json!("{\"test\": true, \"value\": 42}"));
                            params
                        },
                        timeout_ms: Some(5000),
                        retry_count: Some(2),
                        cache_ttl_seconds: Some(300),
                    },
                },
                // UUID generator
                WorkflowNode {
                    id: "uuid_gen".to_string(),
                    node_type: "util.uuid".to_string(),
                    name: "UUID Generator".to_string(),
                    description: Some("Generate a unique ID".to_string()),
                    category: NodeCategory::Utilities,
                    position: NodePosition { x: 500.0, y: 100.0 },
                    configuration: NodeConfiguration {
                        parameters: std::collections::HashMap::new(),
                        timeout_ms: Some(1000),
                        retry_count: Some(1),
                        cache_ttl_seconds: None,
                    },
                },
            ],
            connections: vec![
                WorkflowConnection {
                    id: Uuid::new_v4().to_string(),
                    source_node_id: "trigger_node".to_string(),
                    target_node_id: "json_parser".to_string(),
                    source_output: "data".to_string(),
                    target_input: "json_data".to_string(),
                    condition: None,
                },
                WorkflowConnection {
                    id: Uuid::new_v4().to_string(),
                    source_node_id: "json_parser".to_string(),
                    target_node_id: "uuid_gen".to_string(),
                    source_output: "parsed_data".to_string(),
                    target_input: "trigger".to_string(),
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

    fn create_realistic_input_data() -> std::collections::HashMap<String, serde_json::Value> {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("workflow_input".to_string(), serde_json::json!("test_data"));
        input_data.insert("execution_context".to_string(), serde_json::json!("end_to_end_test"));
        input_data.insert("timestamp".to_string(), serde_json::json!(chrono::Utc::now()));
        input_data
    }

    fn create_trigger_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "trigger.manual".to_string(),
            name: "Manual Trigger".to_string(),
            description: "Manual trigger for testing".to_string(),
            category: NodeCategory::Triggers,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_trigger".to_string(),
        }
    }

    fn create_trigger_context() -> NodeExecutionContext {
        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_trigger".to_string(),
            input_data: std::collections::HashMap::new(),
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    fn create_communication_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "communication.http.request".to_string(),
            name: "HTTP Request".to_string(),
            description: "Make HTTP request".to_string(),
            category: NodeCategory::Communication,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_http_request".to_string(),
        }
    }

    fn create_communication_context() -> NodeExecutionContext {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("url".to_string(), serde_json::json!("https://httpbin.org/get"));
        input_data.insert("method".to_string(), serde_json::json!("GET"));
        
        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_http".to_string(),
            input_data,
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    fn create_processing_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "data.json.parse".to_string(),
            name: "JSON Parser".to_string(),
            description: "Parse JSON data".to_string(),
            category: NodeCategory::Processing,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_json_parse".to_string(),
        }
    }

    fn create_processing_context() -> NodeExecutionContext {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("json_data".to_string(), serde_json::json!("{\"test\": true, \"number\": 42}"));
        
        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_json_parse".to_string(),
            input_data,
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    fn create_ai_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "ai.text.sentiment".to_string(),
            name: "Sentiment Analysis".to_string(),
            description: "Analyze text sentiment".to_string(),
            category: NodeCategory::AI,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_sentiment_analysis".to_string(),
        }
    }

    fn create_ai_context() -> NodeExecutionContext {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("text".to_string(), serde_json::json!("This is a great test!"));
        
        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_sentiment".to_string(),
            input_data,
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    fn create_utility_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "util.uuid".to_string(),
            name: "UUID Generator".to_string(),
            description: "Generate UUID".to_string(),
            category: NodeCategory::Utilities,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_uuid_generation".to_string(),
        }
    }

    fn create_utility_context() -> NodeExecutionContext {
        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_uuid".to_string(),
            input_data: std::collections::HashMap::new(),
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    fn create_workflow_with_failing_node() -> Workflow {
        let mut workflow = create_realistic_workflow();
        workflow.name = "Error Handling Test Workflow".to_string();
        
        // Add a node that will fail
        workflow.nodes.push(WorkflowNode {
            id: "failing_node".to_string(),
            node_type: "nonexistent.node.type".to_string(), // This will cause failure
            name: "Failing Node".to_string(),
            description: Some("This node will fail for testing".to_string()),
            category: NodeCategory::Processing,
            position: NodePosition { x: 700.0, y: 100.0 },
            configuration: NodeConfiguration {
                parameters: std::collections::HashMap::new(),
                timeout_ms: Some(1000),
                retry_count: Some(1),
                cache_ttl_seconds: None,
            },
        });
        
        workflow
    }

    fn create_performance_monitoring_workflow() -> Workflow {
        let mut workflow = create_realistic_workflow();
        workflow.name = "Performance Monitoring Test".to_string();

        // Add more nodes for performance testing
        for i in 1..=3 {
            workflow.nodes.push(WorkflowNode {
                id: format!("perf_node_{}", i),
                node_type: "util.uuid".to_string(),
                name: format!("Performance Node {}", i),
                description: Some("Performance testing node".to_string()),
                category: NodeCategory::Utilities,
                position: NodePosition { x: (600 + i * 100) as f64, y: 100.0 },
                configuration: NodeConfiguration {
                    parameters: std::collections::HashMap::new(),
                    timeout_ms: Some(1000),
                    retry_count: Some(1),
                    cache_ttl_seconds: None,
                },
            });
        }

        workflow
    }

    // Helper functions for OpenRouter and Gemini testing

    fn create_openrouter_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "ai.openrouter.chat".to_string(),
            name: "OpenRouter Chat".to_string(),
            description: "Chat with ANY model on OpenRouter".to_string(),
            category: NodeCategory::AI,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_openrouter_chat".to_string(),
        }
    }

    fn create_openrouter_context() -> NodeExecutionContext {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("prompt".to_string(), serde_json::json!("Hello, how are you?"));
        input_data.insert("model".to_string(), serde_json::json!("anthropic/claude-3.5-sonnet")); // User manually specifies ANY model
        input_data.insert("api_key".to_string(), serde_json::json!("test_api_key")); // Would be real API key in production
        input_data.insert("temperature".to_string(), serde_json::json!(0.7));
        input_data.insert("max_tokens".to_string(), serde_json::json!(100));

        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_openrouter".to_string(),
            input_data,
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    fn create_gemini_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "ai.gemini.chat".to_string(),
            name: "Google Gemini Chat".to_string(),
            description: "Chat with Google Gemini models".to_string(),
            category: NodeCategory::AI,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_gemini_chat".to_string(),
        }
    }

    fn create_gemini_context() -> NodeExecutionContext {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("prompt".to_string(), serde_json::json!("Hello, how are you?"));
        input_data.insert("model".to_string(), serde_json::json!("gemini-2.5-flash"));
        input_data.insert("api_key".to_string(), serde_json::json!("test_api_key")); // Would be real API key in production
        input_data.insert("temperature".to_string(), serde_json::json!(0.7));
        input_data.insert("max_output_tokens".to_string(), serde_json::json!(100));

        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_gemini".to_string(),
            input_data,
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }

    // Helper functions for Chat Trigger testing

    fn create_chat_trigger_node() -> NodeDefinition {
        NodeDefinition {
            node_type: "trigger.chat".to_string(),
            name: "Chat Trigger".to_string(),
            description: "Trigger workflows through natural language chat".to_string(),
            category: NodeCategory::Triggers,
            inputs: vec![],
            outputs: vec![],
            configuration_schema: serde_json::json!({}),
            execution_handler: "execute_chat_trigger".to_string(),
        }
    }

    fn create_chat_trigger_context(message: &str) -> NodeExecutionContext {
        let mut input_data = std::collections::HashMap::new();
        input_data.insert("user_message".to_string(), serde_json::json!(message));
        input_data.insert("conversation_id".to_string(), serde_json::json!("test_conversation"));
        input_data.insert("user_id".to_string(), serde_json::json!("test_user"));

        NodeExecutionContext {
            execution_id: Uuid::new_v4().to_string(),
            node_id: "test_chat_trigger".to_string(),
            input_data,
            configuration: std::collections::HashMap::new(),
            context_data: None,
        }
    }
}
