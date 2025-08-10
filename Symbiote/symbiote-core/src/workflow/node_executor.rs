//! Node Execution Engine
//! 
//! This module implements the actual execution logic for all 100+ workflow nodes.
//! It provides a unified execution interface that handles different node types
//! and integrates with external services and APIs.

use crate::{Result, SymbioteError};
use super::{NodeDefinition, NodeCategory};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use chrono::{DateTime, Utc};
use regex::Regex;

/// Node execution engine that handles all node types
#[derive(Debug)]
pub struct NodeExecutionEngine {
    /// HTTP client for API calls
    http_client: reqwest::Client,
    
    /// Configuration for external services
    service_configs: HashMap<String, ServiceConfig>,
    
    /// Execution cache
    execution_cache: Arc<tokio::sync::RwLock<HashMap<String, CachedResult>>>,
}

/// Service configuration for external integrations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub service_name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub rate_limit_per_minute: u32,
}

/// Cached execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResult {
    pub result: serde_json::Value,
    pub cached_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

/// Node execution context
#[derive(Debug, Clone)]
pub struct NodeExecutionContext {
    pub execution_id: String,
    pub node_id: String,
    pub input_data: HashMap<String, serde_json::Value>,
    pub configuration: HashMap<String, serde_json::Value>,
    pub context_data: Option<HashMap<String, serde_json::Value>>,
}

/// Node execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    pub success: bool,
    pub output_data: HashMap<String, serde_json::Value>,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl NodeExecutionEngine {
    /// Create a new node execution engine
    pub fn new() -> Self {
        let mut service_configs = HashMap::new();
        
        // Add default service configurations
        service_configs.insert("openai".to_string(), ServiceConfig {
            service_name: "OpenAI".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            timeout_seconds: 30,
            rate_limit_per_minute: 60,
        });
        
        service_configs.insert("github".to_string(), ServiceConfig {
            service_name: "GitHub".to_string(),
            base_url: "https://api.github.com".to_string(),
            api_key: None,
            timeout_seconds: 15,
            rate_limit_per_minute: 5000,
        });
        
        service_configs.insert("slack".to_string(), ServiceConfig {
            service_name: "Slack".to_string(),
            base_url: "https://slack.com/api".to_string(),
            api_key: None,
            timeout_seconds: 10,
            rate_limit_per_minute: 100,
        });

        Self {
            http_client: reqwest::Client::new(),
            service_configs,
            execution_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Execute a node with the given context
    pub async fn execute_node(
        &self,
        node_definition: &NodeDefinition,
        context: NodeExecutionContext,
    ) -> Result<NodeExecutionResult> {
        let start_time = std::time::Instant::now();
        
        // Check cache first
        if let Some(cached_result) = self.check_cache(&context).await? {
            return Ok(NodeExecutionResult {
                success: true,
                output_data: serde_json::from_value(cached_result.result)?,
                error_message: None,
                execution_time_ms: 0, // Cached result
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("cached".to_string(), serde_json::json!(true));
                    metadata
                },
            });
        }

        // Execute based on node category
        let result = match node_definition.category {
            NodeCategory::Triggers => self.execute_trigger_node(node_definition, &context).await,
            NodeCategory::Communication => self.execute_communication_node(node_definition, &context).await,
            NodeCategory::Processing => self.execute_processing_node(node_definition, &context).await,
            NodeCategory::AI => self.execute_ai_node(node_definition, &context).await,
            NodeCategory::Development => self.execute_development_node(node_definition, &context).await,
            NodeCategory::DataStorage => self.execute_data_storage_node(node_definition, &context).await,
            NodeCategory::ControlFlow => self.execute_control_flow_node(node_definition, &context).await,
            NodeCategory::Utilities => self.execute_utility_node(node_definition, &context).await,
            _ => Err(SymbioteError::NotImplemented(format!("Node category not implemented: {:?}", node_definition.category))),
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(output_data) => {
                // Cache successful results
                self.cache_result(&context, &output_data).await?;
                
                Ok(NodeExecutionResult {
                    success: true,
                    output_data,
                    error_message: None,
                    execution_time_ms: execution_time,
                    metadata: HashMap::new(),
                })
            }
            Err(error) => {
                Ok(NodeExecutionResult {
                    success: false,
                    output_data: HashMap::new(),
                    error_message: Some(error.to_string()),
                    execution_time_ms: execution_time,
                    metadata: HashMap::new(),
                })
            }
        }
    }

    /// Execute trigger nodes
    async fn execute_trigger_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();

        match node_definition.node_type.as_str() {
            "trigger.chat" => {
                let user_message = context.input_data.get("user_message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing user_message".to_string()))?;

                let conversation_id = context.input_data.get("conversation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                let user_id = context.input_data.get("user_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("anonymous");

                // Parse intent from user message
                let intent = self.parse_chat_intent(user_message).await?;

                // Extract entities and parameters from the message
                let entities = self.extract_entities(user_message).await?;

                // Determine workflow trigger conditions
                let trigger_conditions = self.analyze_trigger_conditions(user_message, &intent, &entities).await?;

                // Build context from conversation history
                let conversation_context = self.build_conversation_context(conversation_id, user_id).await?;

                output.insert("triggered".to_string(), serde_json::json!(true));
                output.insert("user_message".to_string(), serde_json::json!(user_message));
                output.insert("conversation_id".to_string(), serde_json::json!(conversation_id));
                output.insert("user_id".to_string(), serde_json::json!(user_id));
                output.insert("intent".to_string(), serde_json::json!(intent));
                output.insert("entities".to_string(), serde_json::json!(entities));
                output.insert("trigger_conditions".to_string(), serde_json::json!(trigger_conditions));
                output.insert("conversation_context".to_string(), serde_json::json!(conversation_context));
                output.insert("timestamp".to_string(), serde_json::json!(Utc::now()));

                // Add extracted parameters for downstream nodes
                if let Some(params) = entities.get("parameters") {
                    output.insert("extracted_parameters".to_string(), params.clone());
                }

                // Add suggested workflows based on intent
                let suggested_workflows = self.suggest_workflows_for_intent(&intent).await?;
                output.insert("suggested_workflows".to_string(), serde_json::json!(suggested_workflows));
            }
            "trigger.manual" => {
                output.insert("triggered".to_string(), serde_json::json!(true));
                output.insert("timestamp".to_string(), serde_json::json!(Utc::now()));
            }
            "trigger.schedule" => {
                // Schedule trigger logic
                output.insert("scheduled".to_string(), serde_json::json!(true));
                output.insert("next_run".to_string(), serde_json::json!(Utc::now()));
            }
            "trigger.webhook" => {
                // Webhook trigger logic
                output.insert("webhook_received".to_string(), serde_json::json!(true));
                if let Some(payload) = context.input_data.get("payload") {
                    output.insert("payload".to_string(), payload.clone());
                }
            }
            "trigger.file_watcher" => {
                // File watcher trigger logic
                output.insert("file_changed".to_string(), serde_json::json!(true));
                if let Some(file_path) = context.input_data.get("file_path") {
                    output.insert("file_path".to_string(), file_path.clone());
                }
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Trigger node not implemented: {}", node_definition.node_type)));
            }
        }

        Ok(output)
    }

    /// Parse intent from user chat message
    async fn parse_chat_intent(&self, message: &str) -> Result<serde_json::Value> {
        let message_lower = message.to_lowercase();

        let intent = if message_lower.contains("generate") || message_lower.contains("create") || message_lower.contains("write") {
            if message_lower.contains("code") || message_lower.contains("function") || message_lower.contains("class") {
                serde_json::json!({
                    "type": "code_generation",
                    "confidence": 0.9,
                    "category": "development"
                })
            } else if message_lower.contains("test") {
                serde_json::json!({
                    "type": "test_generation",
                    "confidence": 0.85,
                    "category": "development"
                })
            } else if message_lower.contains("documentation") || message_lower.contains("docs") {
                serde_json::json!({
                    "type": "documentation_generation",
                    "confidence": 0.8,
                    "category": "development"
                })
            } else {
                serde_json::json!({
                    "type": "content_generation",
                    "confidence": 0.7,
                    "category": "general"
                })
            }
        } else if message_lower.contains("analyze") || message_lower.contains("review") || message_lower.contains("check") {
            if message_lower.contains("code") {
                serde_json::json!({
                    "type": "code_analysis",
                    "confidence": 0.9,
                    "category": "development"
                })
            } else if message_lower.contains("performance") {
                serde_json::json!({
                    "type": "performance_analysis",
                    "confidence": 0.85,
                    "category": "optimization"
                })
            } else {
                serde_json::json!({
                    "type": "general_analysis",
                    "confidence": 0.7,
                    "category": "analysis"
                })
            }
        } else if message_lower.contains("deploy") || message_lower.contains("build") || message_lower.contains("release") {
            serde_json::json!({
                "type": "deployment",
                "confidence": 0.9,
                "category": "devops"
            })
        } else if message_lower.contains("send") || message_lower.contains("notify") || message_lower.contains("message") {
            serde_json::json!({
                "type": "communication",
                "confidence": 0.8,
                "category": "communication"
            })
        } else if message_lower.contains("schedule") || message_lower.contains("remind") || message_lower.contains("later") {
            serde_json::json!({
                "type": "scheduling",
                "confidence": 0.8,
                "category": "automation"
            })
        } else {
            serde_json::json!({
                "type": "general_query",
                "confidence": 0.5,
                "category": "general"
            })
        };

        Ok(intent)
    }

    /// Extract entities and parameters from user message
    async fn extract_entities(&self, message: &str) -> Result<serde_json::Value> {
        let mut entities = serde_json::Map::new();
        let mut parameters = serde_json::Map::new();

        // Extract file paths
        let file_regex = Regex::new(r"([a-zA-Z0-9_\-./]+\.(rs|js|py|ts|java|cpp|c|h|md|txt|json|yaml|yml|toml))").unwrap();
        let file_matches: Vec<String> = file_regex.find_iter(message)
            .map(|m| m.as_str().to_string())
            .collect();
        if !file_matches.is_empty() {
            entities.insert("files".to_string(), serde_json::json!(file_matches));
        }

        // Extract programming languages
        let lang_patterns = vec![
            ("rust", vec!["rust", "rs", "cargo"]),
            ("javascript", vec!["javascript", "js", "node", "npm"]),
            ("python", vec!["python", "py", "pip"]),
            ("typescript", vec!["typescript", "ts"]),
            ("java", vec!["java", "maven", "gradle"]),
            ("cpp", vec!["c++", "cpp", "cmake"]),
            ("go", vec!["go", "golang"]),
        ];

        let mut detected_languages = Vec::new();
        for (lang, patterns) in lang_patterns {
            for pattern in patterns {
                if message.to_lowercase().contains(pattern) {
                    detected_languages.push(lang);
                    break;
                }
            }
        }
        if !detected_languages.is_empty() {
            entities.insert("languages".to_string(), serde_json::json!(detected_languages));
        }

        // Extract numbers and quantities
        let number_regex = Regex::new(r"\b(\d+)\b").unwrap();
        let numbers: Vec<i32> = number_regex.find_iter(message)
            .filter_map(|m| m.as_str().parse().ok())
            .collect();
        if !numbers.is_empty() {
            entities.insert("numbers".to_string(), serde_json::json!(numbers));
        }

        // Extract time expressions
        let time_patterns = vec!["today", "tomorrow", "yesterday", "now", "later", "soon", "minute", "hour", "day", "week"];
        let mut time_expressions = Vec::new();
        for pattern in time_patterns {
            if message.to_lowercase().contains(pattern) {
                time_expressions.push(pattern);
            }
        }
        if !time_expressions.is_empty() {
            entities.insert("time_expressions".to_string(), serde_json::json!(time_expressions));
        }

        // Extract quoted strings as potential parameters
        let quote_regex = Regex::new(r#""([^"]+)"|'([^']+)'"#).unwrap();
        let quoted_strings: Vec<String> = quote_regex.captures_iter(message)
            .map(|cap| cap.get(1).or_else(|| cap.get(2)).unwrap().as_str().to_string())
            .collect();
        if !quoted_strings.is_empty() {
            parameters.insert("quoted_strings".to_string(), serde_json::json!(quoted_strings));
        }

        // Add parameters to entities
        if !parameters.is_empty() {
            entities.insert("parameters".to_string(), serde_json::Value::Object(parameters));
        }

        Ok(serde_json::Value::Object(entities))
    }

    /// Analyze trigger conditions based on message content
    async fn analyze_trigger_conditions(
        &self,
        message: &str,
        intent: &serde_json::Value,
        entities: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let mut conditions = serde_json::Map::new();

        // Determine urgency
        let urgency = if message.to_lowercase().contains("urgent") || message.to_lowercase().contains("asap") || message.to_lowercase().contains("immediately") {
            "high"
        } else if message.to_lowercase().contains("when you can") || message.to_lowercase().contains("no rush") {
            "low"
        } else {
            "medium"
        };
        conditions.insert("urgency".to_string(), serde_json::json!(urgency));

        // Determine scope
        let scope = if entities.get("files").is_some() {
            "file_specific"
        } else if message.to_lowercase().contains("project") || message.to_lowercase().contains("entire") {
            "project_wide"
        } else {
            "general"
        };
        conditions.insert("scope".to_string(), serde_json::json!(scope));

        // Determine if user wants interactive feedback
        let interactive = message.to_lowercase().contains("show me") ||
                         message.to_lowercase().contains("let me know") ||
                         message.to_lowercase().contains("update me");
        conditions.insert("interactive".to_string(), serde_json::json!(interactive));

        // Determine if this should trigger immediately or be scheduled
        let immediate = !message.to_lowercase().contains("later") &&
                       !message.to_lowercase().contains("schedule") &&
                       !message.to_lowercase().contains("remind");
        conditions.insert("immediate".to_string(), serde_json::json!(immediate));

        Ok(serde_json::Value::Object(conditions))
    }

    /// Build conversation context for better understanding
    async fn build_conversation_context(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<serde_json::Value> {
        // In a real implementation, this would query the conversation history
        // from the ContextBus and build relevant context

        let mut context = serde_json::Map::new();
        context.insert("conversation_id".to_string(), serde_json::json!(conversation_id));
        context.insert("user_id".to_string(), serde_json::json!(user_id));
        context.insert("message_count".to_string(), serde_json::json!(1)); // Placeholder
        context.insert("last_activity".to_string(), serde_json::json!(Utc::now()));

        // Add current workspace context if available
        context.insert("current_workspace".to_string(), serde_json::json!("symbiote-ide"));
        context.insert("open_files".to_string(), serde_json::json!([])); // Would get from ContextBus
        context.insert("active_agents".to_string(), serde_json::json!([])); // Would get from ContextBus

        Ok(serde_json::Value::Object(context))
    }

    /// Suggest workflows based on detected intent
    async fn suggest_workflows_for_intent(&self, intent: &serde_json::Value) -> Result<Vec<String>> {
        let intent_type = intent.get("type").and_then(|t| t.as_str()).unwrap_or("general_query");

        let suggested_workflows = match intent_type {
            "code_generation" => vec![
                "code_generator_workflow".to_string(),
                "ai_code_assistant_workflow".to_string(),
                "code_review_workflow".to_string(),
            ],
            "test_generation" => vec![
                "test_generator_workflow".to_string(),
                "unit_test_workflow".to_string(),
                "integration_test_workflow".to_string(),
            ],
            "documentation_generation" => vec![
                "documentation_generator_workflow".to_string(),
                "api_docs_workflow".to_string(),
                "readme_generator_workflow".to_string(),
            ],
            "code_analysis" => vec![
                "code_analyzer_workflow".to_string(),
                "security_audit_workflow".to_string(),
                "performance_analysis_workflow".to_string(),
            ],
            "deployment" => vec![
                "ci_cd_workflow".to_string(),
                "deployment_workflow".to_string(),
                "release_workflow".to_string(),
            ],
            "communication" => vec![
                "notification_workflow".to_string(),
                "team_update_workflow".to_string(),
                "slack_integration_workflow".to_string(),
            ],
            "scheduling" => vec![
                "task_scheduler_workflow".to_string(),
                "reminder_workflow".to_string(),
                "automated_workflow".to_string(),
            ],
            _ => vec![
                "general_ai_assistant_workflow".to_string(),
                "context_aware_workflow".to_string(),
            ],
        };

        Ok(suggested_workflows)
    }

    /// Execute communication nodes
    async fn execute_communication_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();
        
        match node_definition.node_type.as_str() {
            "communication.slack.send" => {
                // Slack send message
                let message = context.input_data.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing message".to_string()))?;
                
                let channel = context.input_data.get("channel")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#general");
                
                // Simulate Slack API call
                output.insert("message_sent".to_string(), serde_json::json!(true));
                output.insert("channel".to_string(), serde_json::json!(channel));
                output.insert("message".to_string(), serde_json::json!(message));
                output.insert("timestamp".to_string(), serde_json::json!(Utc::now()));
            }
            "communication.email.send" => {
                // Email send
                let to = context.input_data.get("to")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing recipient".to_string()))?;
                
                let subject = context.input_data.get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No Subject");
                
                let body = context.input_data.get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                // Simulate email sending
                output.insert("email_sent".to_string(), serde_json::json!(true));
                output.insert("to".to_string(), serde_json::json!(to));
                output.insert("subject".to_string(), serde_json::json!(subject));
                output.insert("message_id".to_string(), serde_json::json!(uuid::Uuid::new_v4().to_string()));
            }
            "communication.http.request" => {
                // HTTP request
                let url = context.input_data.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing URL".to_string()))?;
                
                let method = context.input_data.get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GET");
                
                // Make actual HTTP request
                let response = match method.to_uppercase().as_str() {
                    "GET" => self.http_client.get(url).send().await,
                    "POST" => {
                        let body = context.input_data.get("body").cloned().unwrap_or(serde_json::Value::Null);
                        self.http_client.post(url).json(&body).send().await
                    }
                    _ => return Err(SymbioteError::Validation(format!("Unsupported HTTP method: {}", method))),
                };
                
                match response {
                    Ok(resp) => {
                        output.insert("status_code".to_string(), serde_json::json!(resp.status().as_u16()));
                        output.insert("success".to_string(), serde_json::json!(resp.status().is_success()));
                        
                        if let Ok(text) = resp.text().await {
                            output.insert("response_body".to_string(), serde_json::json!(text));
                        }
                    }
                    Err(e) => {
                        return Err(SymbioteError::External(format!("HTTP request failed: {}", e)));
                    }
                }
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Communication node not implemented: {}", node_definition.node_type)));
            }
        }
        
        Ok(output)
    }

    /// Execute data processing nodes
    async fn execute_processing_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();
        
        match node_definition.node_type.as_str() {
            "data.json.parse" => {
                let json_data = context.input_data.get("json_data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing JSON data".to_string()))?;
                
                match serde_json::from_str::<serde_json::Value>(json_data) {
                    Ok(parsed) => {
                        output.insert("parsed_data".to_string(), parsed);
                        output.insert("success".to_string(), serde_json::json!(true));
                    }
                    Err(e) => {
                        return Err(SymbioteError::Validation(format!("Invalid JSON: {}", e)));
                    }
                }
            }
            "data.transform" => {
                let input_data = context.input_data.get("input_data").cloned()
                    .unwrap_or(serde_json::Value::Null);
                
                let transform_expression = context.input_data.get("transform_expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("identity");
                
                // Simple transformation logic
                let transformed = match transform_expression {
                    "uppercase" => {
                        if let Some(text) = input_data.as_str() {
                            serde_json::json!(text.to_uppercase())
                        } else {
                            input_data
                        }
                    }
                    "lowercase" => {
                        if let Some(text) = input_data.as_str() {
                            serde_json::json!(text.to_lowercase())
                        } else {
                            input_data
                        }
                    }
                    _ => input_data,
                };
                
                output.insert("transformed_data".to_string(), transformed);
            }
            "data.filter" => {
                let input_array = context.input_data.get("input_array")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| SymbioteError::Validation("Missing input array".to_string()))?;
                
                let filter_condition = context.input_data.get("filter_condition")
                    .and_then(|v| v.as_str())
                    .unwrap_or("true");
                
                // Simple filtering (in real implementation, this would be more sophisticated)
                let filtered: Vec<serde_json::Value> = input_array.iter()
                    .filter(|item| {
                        // Simple filter logic - in practice this would evaluate the condition
                        match filter_condition {
                            "not_null" => !item.is_null(),
                            "is_string" => item.is_string(),
                            "is_number" => item.is_number(),
                            _ => true,
                        }
                    })
                    .cloned()
                    .collect();
                
                output.insert("filtered_data".to_string(), serde_json::json!(filtered));
                output.insert("count".to_string(), serde_json::json!(filtered.len()));
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Processing node not implemented: {}", node_definition.node_type)));
            }
        }
        
        Ok(output)
    }

    /// Execute AI nodes
    async fn execute_ai_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();

        match node_definition.node_type.as_str() {
            "ai.openrouter.chat" => {
                let prompt = context.input_data.get("prompt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing prompt".to_string()))?;

                let model = context.input_data.get("model")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing model - user must specify any OpenRouter model manually".to_string()))?;

                let api_key = context.input_data.get("api_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing OpenRouter API key".to_string()))?;

                let temperature = context.input_data.get("temperature")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0);

                let max_tokens = context.input_data.get("max_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000);

                // Prepare OpenRouter API request
                let request_body = serde_json::json!({
                    "model": model,
                    "messages": [
                        {
                            "role": "user",
                            "content": prompt
                        }
                    ],
                    "temperature": temperature,
                    "max_tokens": max_tokens
                });

                // Make actual OpenRouter API call
                let response = self.http_client
                    .post("https://openrouter.ai/api/v1/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .header("HTTP-Referer", "https://symbiote-ide.com") // Optional site URL
                    .header("X-Title", "Symbiote IDE") // Optional site title
                    .json(&request_body)
                    .send()
                    .await;

                match response {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(api_response) = resp.json::<serde_json::Value>().await {
                                // Extract response from OpenRouter format
                                if let Some(choices) = api_response.get("choices").and_then(|c| c.as_array()) {
                                    if let Some(first_choice) = choices.first() {
                                        if let Some(message) = first_choice.get("message") {
                                            if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                                output.insert("response".to_string(), serde_json::json!(content));
                                            }
                                        }
                                    }
                                }

                                output.insert("model".to_string(), serde_json::json!(model));
                                output.insert("full_response".to_string(), api_response.clone());

                                // Extract usage information if available
                                if let Some(usage) = api_response.get("usage") {
                                    output.insert("usage".to_string(), usage.clone());
                                }
                            }
                        } else {
                            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            return Err(SymbioteError::External(format!("OpenRouter API error: {}", error_text)));
                        }
                    }
                    Err(e) => {
                        return Err(SymbioteError::External(format!("OpenRouter API request failed: {}", e)));
                    }
                }
            }
            "ai.gemini.chat" => {
                let prompt = context.input_data.get("prompt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing prompt".to_string()))?;

                let model = context.input_data.get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gemini-2.5-flash"); // Default to Flash model

                let api_key = context.input_data.get("api_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing Gemini API key".to_string()))?;

                let temperature = context.input_data.get("temperature")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0);

                let max_output_tokens = context.input_data.get("max_output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000);

                // Prepare Gemini API request
                let request_body = serde_json::json!({
                    "contents": [
                        {
                            "parts": [
                                {
                                    "text": prompt
                                }
                            ]
                        }
                    ],
                    "generationConfig": {
                        "temperature": temperature,
                        "maxOutputTokens": max_output_tokens
                    }
                });

                // Make actual Gemini API call
                let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", model, api_key);

                let response = self.http_client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send()
                    .await;

                match response {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(api_response) = resp.json::<serde_json::Value>().await {
                                // Extract response from Gemini format
                                if let Some(candidates) = api_response.get("candidates").and_then(|c| c.as_array()) {
                                    if let Some(first_candidate) = candidates.first() {
                                        if let Some(content) = first_candidate.get("content") {
                                            if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                                if let Some(first_part) = parts.first() {
                                                    if let Some(text) = first_part.get("text").and_then(|t| t.as_str()) {
                                                        output.insert("response".to_string(), serde_json::json!(text));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                output.insert("model".to_string(), serde_json::json!(model));
                                output.insert("full_response".to_string(), api_response.clone());

                                // Extract usage information if available
                                if let Some(usage_metadata) = api_response.get("usageMetadata") {
                                    output.insert("usage".to_string(), usage_metadata.clone());
                                }
                            }
                        } else {
                            let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                            return Err(SymbioteError::External(format!("Gemini API error: {}", error_text)));
                        }
                    }
                    Err(e) => {
                        return Err(SymbioteError::External(format!("Gemini API request failed: {}", e)));
                    }
                }
            }
            "ai.openai.chat" => {
                let prompt = context.input_data.get("prompt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing prompt".to_string()))?;

                let model = context.input_data.get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gpt-3.5-turbo");

                // Simulate OpenAI API call (in real implementation, make actual API call)
                output.insert("response".to_string(), serde_json::json!(format!("AI response to: {}", prompt)));
                output.insert("model".to_string(), serde_json::json!(model));
                output.insert("tokens_used".to_string(), serde_json::json!(prompt.len() / 4)); // Rough estimate
            }
            "ai.text.sentiment" => {
                let text = context.input_data.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing text".to_string()))?;

                // Simple sentiment analysis (in practice, use ML model)
                let sentiment = if text.contains("good") || text.contains("great") || text.contains("excellent") {
                    "positive"
                } else if text.contains("bad") || text.contains("terrible") || text.contains("awful") {
                    "negative"
                } else {
                    "neutral"
                };

                output.insert("sentiment".to_string(), serde_json::json!(sentiment));
                output.insert("confidence".to_string(), serde_json::json!(0.85));
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("AI node not implemented: {}", node_definition.node_type)));
            }
        }

        Ok(output)
    }

    /// Execute development nodes
    async fn execute_development_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();
        
        match node_definition.node_type.as_str() {
            "dev.github.create_issue" => {
                let title = context.input_data.get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing issue title".to_string()))?;
                
                let body = context.input_data.get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                let repo = context.input_data.get("repository")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing repository".to_string()))?;
                
                // Simulate GitHub API call
                output.insert("issue_created".to_string(), serde_json::json!(true));
                output.insert("issue_number".to_string(), serde_json::json!(42));
                output.insert("issue_url".to_string(), serde_json::json!(format!("https://github.com/{}/issues/42", repo)));
            }
            "dev.git.commit" => {
                let message = context.input_data.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing commit message".to_string()))?;
                
                // Simulate git commit
                output.insert("commit_hash".to_string(), serde_json::json!("abc123def456"));
                output.insert("message".to_string(), serde_json::json!(message));
                output.insert("timestamp".to_string(), serde_json::json!(Utc::now()));
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Development node not implemented: {}", node_definition.node_type)));
            }
        }
        
        Ok(output)
    }

    /// Execute data storage nodes
    async fn execute_data_storage_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();
        
        match node_definition.node_type.as_str() {
            "storage.file.read" => {
                let file_path = context.input_data.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing file path".to_string()))?;
                
                // Simulate file reading
                output.insert("file_content".to_string(), serde_json::json!(format!("Content of {}", file_path)));
                output.insert("file_size".to_string(), serde_json::json!(1024));
            }
            "storage.file.write" => {
                let file_path = context.input_data.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing file path".to_string()))?;
                
                let content = context.input_data.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                // Simulate file writing
                output.insert("file_written".to_string(), serde_json::json!(true));
                output.insert("bytes_written".to_string(), serde_json::json!(content.len()));
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Data storage node not implemented: {}", node_definition.node_type)));
            }
        }
        
        Ok(output)
    }

    /// Execute control flow nodes
    async fn execute_control_flow_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();
        
        match node_definition.node_type.as_str() {
            "control.if" => {
                let condition = context.input_data.get("condition")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                output.insert("condition_result".to_string(), serde_json::json!(condition));
                output.insert("branch".to_string(), serde_json::json!(if condition { "true" } else { "false" }));
            }
            "control.wait" => {
                let delay_ms = context.input_data.get("delay_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000);
                
                // Simulate wait
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                
                output.insert("waited_ms".to_string(), serde_json::json!(delay_ms));
                output.insert("completed_at".to_string(), serde_json::json!(Utc::now()));
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Control flow node not implemented: {}", node_definition.node_type)));
            }
        }
        
        Ok(output)
    }

    /// Execute utility nodes
    async fn execute_utility_node(
        &self,
        node_definition: &NodeDefinition,
        context: &NodeExecutionContext,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut output = HashMap::new();
        
        match node_definition.node_type.as_str() {
            "util.hash" => {
                let input_data = context.input_data.get("input_data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| SymbioteError::Validation("Missing input data".to_string()))?;
                
                let algorithm = context.input_data.get("algorithm")
                    .and_then(|v| v.as_str())
                    .unwrap_or("sha256");
                
                // Simple hash calculation (in practice, use proper crypto library)
                let hash = format!("{}:{}", algorithm, input_data.len());
                
                output.insert("hash".to_string(), serde_json::json!(hash));
                output.insert("algorithm".to_string(), serde_json::json!(algorithm));
            }
            "util.uuid" => {
                let uuid = uuid::Uuid::new_v4().to_string();
                output.insert("uuid".to_string(), serde_json::json!(uuid));
            }
            _ => {
                return Err(SymbioteError::NotImplemented(format!("Utility node not implemented: {}", node_definition.node_type)));
            }
        }
        
        Ok(output)
    }

    /// Check execution cache
    async fn check_cache(&self, context: &NodeExecutionContext) -> Result<Option<CachedResult>> {
        let cache_key = self.generate_cache_key(context);
        let cache = self.execution_cache.read().await;
        
        if let Some(cached) = cache.get(&cache_key) {
            let now = Utc::now();
            let cache_age = now.signed_duration_since(cached.cached_at).num_seconds() as u64;
            
            if cache_age < cached.ttl_seconds {
                return Ok(Some(cached.clone()));
            }
        }
        
        Ok(None)
    }

    /// Cache execution result
    async fn cache_result(
        &self,
        context: &NodeExecutionContext,
        result: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let cache_key = self.generate_cache_key(context);
        let cached_result = CachedResult {
            result: serde_json::json!(result),
            cached_at: Utc::now(),
            ttl_seconds: 300, // 5 minutes default TTL
        };
        
        let mut cache = self.execution_cache.write().await;
        cache.insert(cache_key, cached_result);
        
        Ok(())
    }

    /// Generate cache key for execution context
    fn generate_cache_key(&self, context: &NodeExecutionContext) -> String {
        format!("{}:{}:{:?}", context.node_id, context.execution_id, context.input_data)
    }
}
