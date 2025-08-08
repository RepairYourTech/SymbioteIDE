// User Agent Builder - Visual System for Users to Create Custom Agents
// Phase 3 Feature: Visual workflow/agent builder for users to create deployable agents

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// User Agent Builder - System for users to visually create custom agents and workflows
pub struct UserAgentBuilder {
    user_agents: Arc<RwLock<HashMap<String, UserAgent>>>,
    agent_canvases: Arc<RwLock<HashMap<String, AgentCanvas>>>,
    node_library: Arc<RwLock<AgentNodeLibrary>>,
    agent_generator: AgentCodeGenerator,
    agent_runtime: UserAgentRuntime,
    metrics: Arc<RwLock<UserAgentBuilderMetrics>>,
}

/// User-created agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAgent {
    pub agent_id: String,
    pub agent_name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub agent_type: UserAgentType,
    pub workflow: AgentWorkflow,
    pub configuration: AgentConfiguration,
    pub status: UserAgentStatus,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserAgentType {
    TaskAutomation,
    DataProcessor,
    ContentGenerator,
    CodeAssistant,
    ChatBot,
    APIIntegrator,
    Custom(String),
}

/// Agent workflow (visual flow of nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkflow {
    pub workflow_id: String,
    pub nodes: HashMap<String, AgentNode>,
    pub connections: Vec<NodeConnection>,
    pub entry_points: Vec<String>,
    pub variables: HashMap<String, WorkflowVariable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNode {
    pub node_id: String,
    pub node_type: AgentNodeType,
    pub node_name: String,
    pub position: NodePosition,
    pub configuration: NodeConfiguration,
    pub inputs: HashMap<String, NodeInput>,
    pub outputs: HashMap<String, NodeOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentNodeType {
    // Input/Output nodes
    Input(InputType),
    Output(OutputType),
    
    // AI/ML nodes
    LLMCall(LLMConfiguration),
    ImageGeneration,
    TextToSpeech,
    
    // Processing nodes
    Transform,
    Filter,
    Validate,
    
    // Logic nodes
    Condition,
    Loop,
    Branch,
    
    // Integration nodes
    APICall(APIConfiguration),
    DatabaseQuery,
    FileOperation,
    
    // Communication nodes
    SendMessage,
    SendEmail,
    WebhookCall,
    
    // Custom nodes
    CustomFunction(String),
    SubAgent(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputType {
    Text,
    Number,
    File,
    Image,
    JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputType {
    Text,
    Number,
    File,
    JSON,
    Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfiguration {
    pub provider: LLMProvider,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Google,
    Local(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIConfiguration {
    pub endpoint: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub authentication: AuthenticationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationType {
    None,
    ApiKey(String),
    Bearer(String),
    OAuth2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfiguration {
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout: Option<std::time::Duration>,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryPolicy {
    None,
    Fixed { attempts: u32, delay: std::time::Duration },
    Exponential { attempts: u32, base_delay: std::time::Duration },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInput {
    pub input_name: String,
    pub input_type: DataType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    pub output_name: String,
    pub output_type: DataType,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    File,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConnection {
    pub connection_id: String,
    pub source_node: String,
    pub source_output: String,
    pub target_node: String,
    pub target_input: String,
    pub conditions: Vec<ConnectionCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionCondition {
    pub condition_type: ConditionType,
    pub expression: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    Always,
    OnSuccess,
    OnFailure,
    OnValue(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowVariable {
    pub variable_name: String,
    pub variable_type: DataType,
    pub value: serde_json::Value,
    pub description: String,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfiguration {
    pub execution_mode: ExecutionMode,
    pub resource_limits: ResourceLimits,
    pub security_settings: AgentSecuritySettings,
    pub retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    Synchronous,
    Asynchronous,
    Streaming,
    Batch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory: u64,
    pub max_cpu_time: std::time::Duration,
    pub max_network_requests: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSecuritySettings {
    pub sandbox_mode: bool,
    pub allowed_domains: Vec<String>,
    pub data_retention_policy: DataRetentionPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataRetentionPolicy {
    NoRetention,
    Session,
    Days(u32),
    Permanent,
}

/// Agent canvas for visual editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCanvas {
    pub canvas_id: String,
    pub agent_id: String,
    pub zoom_level: f32,
    pub pan_offset: PanOffset,
    pub selection: Vec<String>,
    pub view_mode: ViewMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanOffset {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewMode {
    Design,
    Code,
    Debug,
    Preview,
}

/// Node library for agent building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNodeLibrary {
    pub categories: HashMap<String, NodeCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCategory {
    pub category_id: String,
    pub category_name: String,
    pub description: String,
    pub icon: String,
    pub nodes: Vec<NodeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub node_type: String,
    pub display_name: String,
    pub description: String,
    pub icon: String,
    pub inputs: Vec<NodeInputDefinition>,
    pub outputs: Vec<NodeOutputDefinition>,
    pub configuration_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInputDefinition {
    pub input_name: String,
    pub input_type: DataType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutputDefinition {
    pub output_name: String,
    pub output_type: DataType,
    pub description: String,
}

/// Agent code generator
pub struct AgentCodeGenerator;

/// Runtime for testing user agents
pub struct UserAgentRuntime {
    active_tests: Arc<RwLock<HashMap<String, AgentTestSession>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTestSession {
    pub session_id: String,
    pub agent_id: String,
    pub test_inputs: HashMap<String, serde_json::Value>,
    pub status: TestStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserAgentStatus {
    Draft,
    Testing,
    Ready,
    Deployed,
    Published,
}

/// Performance metrics
#[derive(Debug, Default)]
pub struct UserAgentBuilderMetrics {
    pub total_agents_created: u32,
    pub agents_deployed: u32,
    pub total_executions: u32,
    pub popular_node_types: HashMap<String, u32>,
}

impl UserAgentBuilder {
    /// Create a new User Agent Builder
    pub fn new() -> Self {
        Self {
            user_agents: Arc::new(RwLock::new(HashMap::new())),
            agent_canvases: Arc::new(RwLock::new(HashMap::new())),
            node_library: Arc::new(RwLock::new(AgentNodeLibrary {
                categories: HashMap::new(),
            })),
            agent_generator: AgentCodeGenerator,
            agent_runtime: UserAgentRuntime::new(),
            metrics: Arc::new(RwLock::new(UserAgentBuilderMetrics::default())),
        }
    }
    
    /// Initialize the node library with default nodes
    pub async fn initialize_node_library(&self) -> Result<(), UserAgentBuilderError> {
        let mut library = self.node_library.write().await;
        
        // AI/ML category
        let ai_category = NodeCategory {
            category_id: "ai".to_string(),
            category_name: "AI & ML".to_string(),
            description: "Artificial Intelligence and Machine Learning nodes".to_string(),
            icon: "🧠".to_string(),
            nodes: vec![
                NodeDefinition {
                    node_type: "llm_call".to_string(),
                    display_name: "LLM Call".to_string(),
                    description: "Call a Large Language Model".to_string(),
                    icon: "💬".to_string(),
                    inputs: vec![
                        NodeInputDefinition {
                            input_name: "prompt".to_string(),
                            input_type: DataType::String,
                            description: "The prompt to send to the LLM".to_string(),
                            required: true,
                            default_value: None,
                        }
                    ],
                    outputs: vec![
                        NodeOutputDefinition {
                            output_name: "response".to_string(),
                            output_type: DataType::String,
                            description: "The LLM's response".to_string(),
                        }
                    ],
                    configuration_schema: serde_json::json!({
                        "provider": {"type": "string", "enum": ["openai", "anthropic", "google"]},
                        "model": {"type": "string"},
                        "temperature": {"type": "number", "min": 0.0, "max": 2.0}
                    }),
                }
            ],
        };
        
        library.categories.insert("ai".to_string(), ai_category);
        Ok(())
    }
    
    /// Create a new user agent
    pub async fn create_agent(
        &self,
        name: String,
        description: String,
        agent_type: UserAgentType,
        author: String,
    ) -> Result<String, UserAgentBuilderError> {
        let agent_id = Uuid::new_v4().to_string();
        
        let agent = UserAgent {
            agent_id: agent_id.clone(),
            agent_name: name,
            description,
            version: "1.0.0".to_string(),
            author,
            agent_type,
            workflow: AgentWorkflow {
                workflow_id: Uuid::new_v4().to_string(),
                nodes: HashMap::new(),
                connections: Vec::new(),
                entry_points: Vec::new(),
                variables: HashMap::new(),
            },
            configuration: AgentConfiguration {
                execution_mode: ExecutionMode::Synchronous,
                resource_limits: ResourceLimits {
                    max_memory: 512 * 1024 * 1024, // 512MB
                    max_cpu_time: std::time::Duration::from_secs(300),
                    max_network_requests: 100,
                },
                security_settings: AgentSecuritySettings {
                    sandbox_mode: true,
                    allowed_domains: Vec::new(),
                    data_retention_policy: DataRetentionPolicy::Session,
                },
                retry_policy: RetryPolicy::None,
            },
            status: UserAgentStatus::Draft,
            created_at: Utc::now(),
            last_modified: Utc::now(),
        };
        
        // Store agent
        {
            let mut agents = self.user_agents.write().await;
            agents.insert(agent_id.clone(), agent);
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_agents_created += 1;
        }
        
        Ok(agent_id)
    }
    
    /// Generate code for a user agent
    pub async fn generate_agent_code(
        &self,
        agent_id: String,
        target_language: CodeLanguage,
    ) -> Result<GeneratedAgentCode, UserAgentBuilderError> {
        let agent = {
            let agents = self.user_agents.read().await;
            agents.get(&agent_id)
                .ok_or(UserAgentBuilderError::AgentNotFound)?
                .clone()
        };
        
        let code = match target_language {
            CodeLanguage::JavaScript => format!(
                "// Generated Agent: {}\nclass {} {{\n  async execute(input) {{\n    // Agent logic here\n    return input;\n  }}\n}}",
                agent.agent_name, agent.agent_name.replace(" ", "")
            ),
            CodeLanguage::Python => format!(
                "# Generated Agent: {}\nclass {}:\n    async def execute(self, input):\n        # Agent logic here\n        return input",
                agent.agent_name, agent.agent_name.replace(" ", "")
            ),
            _ => "// Code generation not implemented".to_string(),
        };
        
        Ok(GeneratedAgentCode {
            agent_id: agent.agent_id.clone(),
            language: target_language,
            main_code: code,
            dependencies: vec!["uuid".to_string()],
        })
    }
    
    /// Test a user agent
    pub async fn test_agent(
        &self,
        agent_id: String,
        test_inputs: HashMap<String, serde_json::Value>,
    ) -> Result<String, UserAgentBuilderError> {
        let session_id = self.agent_runtime.start_test_session(agent_id, test_inputs).await?;
        Ok(session_id)
    }
    
    /// Get user agent
    pub async fn get_agent(&self, agent_id: String) -> Option<UserAgent> {
        let agents = self.user_agents.read().await;
        agents.get(&agent_id).cloned()
    }
    
    /// List user agents
    pub async fn list_agents(&self) -> Vec<UserAgent> {
        let agents = self.user_agents.read().await;
        agents.values().cloned().collect()
    }
}

impl UserAgentRuntime {
    pub fn new() -> Self {
        Self {
            active_tests: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn start_test_session(
        &self,
        agent_id: String,
        test_inputs: HashMap<String, serde_json::Value>,
    ) -> Result<String, UserAgentBuilderError> {
        let session_id = Uuid::new_v4().to_string();
        
        let session = AgentTestSession {
            session_id: session_id.clone(),
            agent_id,
            test_inputs,
            status: TestStatus::Running,
            started_at: Utc::now(),
            completed_at: None,
        };
        
        let mut tests = self.active_tests.write().await;
        tests.insert(session_id.clone(), session);
        
        Ok(session_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CodeLanguage {
    JavaScript,
    Python,
    Rust,
    TypeScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAgentCode {
    pub agent_id: String,
    pub language: CodeLanguage,
    pub main_code: String,
    pub dependencies: Vec<String>,
}

/// User Agent Builder error types
#[derive(Debug, thiserror::Error)]
pub enum UserAgentBuilderError {
    #[error("Agent not found")]
    AgentNotFound,
    #[error("Code generation failed")]
    CodeGenerationFailed,
    #[error("Test execution failed")]
    TestExecutionFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_user_agent_creation() {
        let builder = UserAgentBuilder::new();
        builder.initialize_node_library().await.unwrap();
        
        let agent_id = builder.create_agent(
            "Test Agent".to_string(),
            "A test agent".to_string(),
            UserAgentType::TaskAutomation,
            "test_user".to_string(),
        ).await.unwrap();
        
        assert!(!agent_id.is_empty());
        
        let agent = builder.get_agent(agent_id).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().agent_name, "Test Agent");
    }
    
    #[tokio::test]
    async fn test_agent_code_generation() {
        let builder = UserAgentBuilder::new();
        
        let agent_id = builder.create_agent(
            "Code Gen Test".to_string(),
            "Test code generation".to_string(),
            UserAgentType::CodeAssistant,
            "test_user".to_string(),
        ).await.unwrap();
        
        let generated = builder.generate_agent_code(
            agent_id,
            CodeLanguage::JavaScript,
        ).await.unwrap();
        
        assert!(generated.main_code.contains("CodeGenTest"));
        assert!(!generated.dependencies.is_empty());
    }
}
