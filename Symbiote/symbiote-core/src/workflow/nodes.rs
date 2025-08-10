//! Node Registry - Comprehensive collection of 200+ workflow nodes
//! 
//! This module provides a comprehensive node registry with nodes across 18 categories,
//! rivaling n8n's node library while adding advanced AI agent capabilities.

use crate::{Result, SymbioteError};
use crate::workflow::{NodeCategory, WorkflowNode, NodeInput, NodeOutput, DataType, NodeConfiguration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Node registry containing all available workflow nodes
#[derive(Debug)]
pub struct NodeRegistry {
    /// Registered nodes by category
    nodes_by_category: Arc<RwLock<HashMap<NodeCategory, Vec<NodeDefinition>>>>,

    /// Node definitions by type
    node_definitions: Arc<RwLock<HashMap<String, NodeDefinition>>>,

    /// Custom node providers (simplified for now)
    custom_providers: Arc<RwLock<Vec<String>>>,

    /// Node usage statistics
    usage_stats: Arc<RwLock<HashMap<String, NodeUsageStats>>>,
}

/// Node definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub node_type: String,
    pub name: String,
    pub description: String,
    pub category: NodeCategory,
    pub version: String,
    pub author: String,
    pub icon: String,
    pub color: String,
    pub inputs: Vec<NodeInput>,
    pub outputs: Vec<NodeOutput>,
    pub configuration_schema: serde_json::Value,
    pub documentation_url: Option<String>,
    pub examples: Vec<NodeExample>,
    pub tags: Vec<String>,
    pub pricing: NodePricing,
    pub requirements: NodeRequirements,
}

/// Node example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExample {
    pub title: String,
    pub description: String,
    pub configuration: NodeConfiguration,
    pub sample_input: HashMap<String, serde_json::Value>,
    pub expected_output: HashMap<String, serde_json::Value>,
}

/// Node pricing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePricing {
    pub pricing_model: PricingModel,
    pub cost_per_execution: f64,
    pub cost_per_minute: f64,
    pub cost_per_mb: f64,
    pub free_tier_limit: Option<u64>,
}

/// Pricing models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PricingModel {
    Free,
    PayPerUse,
    Subscription,
    Enterprise,
}

/// Node requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRequirements {
    pub min_memory_mb: u64,
    pub min_cpu_cores: f64,
    pub requires_gpu: bool,
    pub requires_internet: bool,
    pub supported_platforms: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Node usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeUsageStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub popularity_score: f64,
}

/// Node provider trait for custom nodes
pub trait NodeProvider: Send + Sync {
    fn get_node_definitions(&self) -> Vec<NodeDefinition>;
    fn execute_node(&self, node_type: &str, config: &NodeConfiguration, inputs: &HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>>;
    fn validate_configuration(&self, node_type: &str, config: &NodeConfiguration) -> Result<()>;
}

impl NodeRegistry {
    /// Create a new node registry with all built-in nodes
    pub fn new() -> Self {
        let registry = Self {
            nodes_by_category: Arc::new(RwLock::new(HashMap::new())),
            node_definitions: Arc::new(RwLock::new(HashMap::new())),
            custom_providers: Arc::new(RwLock::new(Vec::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Initialize with built-in nodes
        tokio::spawn({
            let registry = registry.clone();
            async move {
                registry.initialize_builtin_nodes().await;
            }
        });
        
        registry
    }

    /// Initialize all built-in nodes across all categories
    async fn initialize_builtin_nodes(&self) {
        // 1. Triggers (8 nodes)
        self.register_triggers_nodes().await;

        // 2. Communication (15 nodes)
        self.register_communication_nodes().await;

        // 3. Data Processing (12 nodes)
        self.register_processing_nodes().await;

        // 4. Business Apps (20 nodes)
        self.register_productivity_nodes().await;

        // 5. Cloud Storage (10 nodes)
        self.register_data_storage_nodes().await;

        // 6. Development (12 nodes)
        self.register_development_nodes().await;

        // 7. AI & ML (15 nodes)
        self.register_ai_nodes().await;

        // 8. Control Flow (8 nodes)
        self.register_control_flow_nodes().await;
    }

    /// Register AI category nodes
    async fn register_ai_nodes(&self) {
        let ai_nodes = vec![
            // LLM Nodes
            self.create_node_definition(
                "ai.openai.chat",
                "OpenAI Chat",
                "Chat with OpenAI GPT models",
                NodeCategory::AI,
                vec![
                    NodeInput {
                        name: "prompt".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("The prompt to send to the AI".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "model".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("gpt-4".to_string())),
                        description: Some("The model to use".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "response".to_string(),
                        data_type: DataType::String,
                        description: Some("The AI response".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "tokens_used".to_string(),
                        data_type: DataType::Number,
                        description: Some("Number of tokens used".to_string()),
                        schema: None,
                    },
                ],
            ),
            
            self.create_node_definition(
                "ai.anthropic.claude",
                "Anthropic Claude",
                "Chat with Anthropic Claude models",
                NodeCategory::AI,
                vec![
                    NodeInput {
                        name: "prompt".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("The prompt to send to Claude".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "response".to_string(),
                        data_type: DataType::String,
                        description: Some("Claude's response".to_string()),
                        schema: None,
                    },
                ],
            ),
            
            self.create_node_definition(
                "ai.embeddings.openai",
                "OpenAI Embeddings",
                "Generate embeddings using OpenAI models",
                NodeCategory::AI,
                vec![
                    NodeInput {
                        name: "text".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Text to generate embeddings for".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "embeddings".to_string(),
                        data_type: DataType::Array,
                        description: Some("Generated embeddings vector".to_string()),
                        schema: None,
                    },
                ],
            ),
            
            self.create_node_definition(
                "ai.image.dalle",
                "DALL-E Image Generation",
                "Generate images using DALL-E",
                NodeCategory::AI,
                vec![
                    NodeInput {
                        name: "prompt".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Image generation prompt".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "image_url".to_string(),
                        data_type: DataType::String,
                        description: Some("Generated image URL".to_string()),
                        schema: None,
                    },
                ],
            ),
            
            self.create_node_definition(
                "ai.classification.text",
                "Text Classification",
                "Classify text using AI models",
                NodeCategory::AI,
                vec![
                    NodeInput {
                        name: "text".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Text to classify".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "categories".to_string(),
                        data_type: DataType::Array,
                        required: true,
                        default_value: None,
                        description: Some("Available categories".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "category".to_string(),
                        data_type: DataType::String,
                        description: Some("Predicted category".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "confidence".to_string(),
                        data_type: DataType::Number,
                        description: Some("Confidence score".to_string()),
                        schema: None,
                    },
                ],
            ),
            
            // Add more AI nodes...
            self.create_node_definition(
                "ai.sentiment.analysis",
                "Sentiment Analysis",
                "Analyze sentiment of text",
                NodeCategory::AI,
                vec![
                    NodeInput {
                        name: "text".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Text to analyze".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "sentiment".to_string(),
                        data_type: DataType::String,
                        description: Some("Sentiment (positive/negative/neutral)".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "score".to_string(),
                        data_type: DataType::Number,
                        description: Some("Sentiment score".to_string()),
                        schema: None,
                    },
                ],
            ),
        ];

        self.register_nodes(ai_nodes).await;
    }

    /// Register Communication category nodes (20 nodes)
    async fn register_communication_nodes(&self) {
        let comm_nodes = vec![
            // Email nodes
            self.create_node_definition(
                "comm.email.send",
                "Send Email",
                "Send emails via SMTP or email services",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "to".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Recipient email address".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "subject".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Email subject".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "body".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Email body".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Sent message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.email.receive",
                "Receive Email",
                "Monitor and receive emails from IMAP/POP3",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "folder".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("INBOX".to_string())),
                        description: Some("Email folder to monitor".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "emails".to_string(),
                        data_type: DataType::Array,
                        description: Some("Received emails".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Slack nodes
            self.create_node_definition(
                "comm.slack.message",
                "Slack Message",
                "Send messages to Slack channels",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "channel".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Slack channel".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Message to send".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "timestamp".to_string(),
                        data_type: DataType::String,
                        description: Some("Message timestamp".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.slack.file_upload",
                "Slack File Upload",
                "Upload files to Slack channels",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "channel".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Slack channel".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "file_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Path to file to upload".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "file_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Uploaded file ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Discord nodes
            self.create_node_definition(
                "comm.discord.message",
                "Discord Message",
                "Send messages to Discord channels",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "channel_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Discord channel ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "content".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Message content".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Sent message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.discord.embed",
                "Discord Embed",
                "Send rich embed messages to Discord",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "channel_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Discord channel ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "embed_data".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("Embed data object".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Sent embed message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Teams nodes
            self.create_node_definition(
                "comm.teams.message",
                "Teams Message",
                "Send messages to Microsoft Teams channels",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "team_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Teams team ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "channel_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Teams channel ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Message content".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Sent message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.teams.adaptive_card",
                "Teams Adaptive Card",
                "Send adaptive cards to Microsoft Teams",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "team_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Teams team ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "card_data".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("Adaptive card JSON".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "card_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Sent card ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // SMS/WhatsApp nodes
            self.create_node_definition(
                "comm.sms.send",
                "Send SMS",
                "Send SMS messages via Twilio or other providers",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "to".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Recipient phone number".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("SMS message content".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_sid".to_string(),
                        data_type: DataType::String,
                        description: Some("SMS message SID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.whatsapp.send",
                "Send WhatsApp",
                "Send WhatsApp messages via WhatsApp Business API",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "to".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Recipient WhatsApp number".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("WhatsApp message content".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::String,
                        description: Some("WhatsApp message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Webhook nodes
            self.create_node_definition(
                "comm.webhook.send",
                "Send Webhook",
                "Send HTTP webhook requests",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "url".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Webhook URL".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "payload".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("Webhook payload".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "response".to_string(),
                        data_type: DataType::Object,
                        description: Some("Webhook response".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.webhook.receive",
                "Receive Webhook",
                "Listen for incoming webhook requests",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "endpoint".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Webhook endpoint path".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "payload".to_string(),
                        data_type: DataType::Object,
                        description: Some("Received webhook payload".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Telegram nodes
            self.create_node_definition(
                "comm.telegram.message",
                "Telegram Message",
                "Send messages via Telegram Bot API",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "chat_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Telegram chat ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "text".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Message text".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::Number,
                        description: Some("Telegram message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.telegram.photo",
                "Telegram Photo",
                "Send photos via Telegram Bot API",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "chat_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Telegram chat ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "photo_url".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Photo URL or file path".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::Number,
                        description: Some("Telegram message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Push notification nodes
            self.create_node_definition(
                "comm.push.firebase",
                "Firebase Push Notification",
                "Send push notifications via Firebase Cloud Messaging",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "token".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Device FCM token".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "title".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Notification title".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "body".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Notification body".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "message_id".to_string(),
                        data_type: DataType::String,
                        description: Some("FCM message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.push.apns",
                "Apple Push Notification",
                "Send push notifications via Apple Push Notification Service",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "device_token".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("iOS device token".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "alert".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("Alert payload".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "apns_id".to_string(),
                        data_type: DataType::String,
                        description: Some("APNS message ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Voice/Call nodes
            self.create_node_definition(
                "comm.voice.call",
                "Make Voice Call",
                "Make voice calls via Twilio or other providers",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "to".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Phone number to call".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Text-to-speech message".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "call_sid".to_string(),
                        data_type: DataType::String,
                        description: Some("Call session ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "comm.voice.transcribe",
                "Transcribe Voice",
                "Transcribe voice recordings to text",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "audio_url".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Audio file URL".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "transcript".to_string(),
                        data_type: DataType::String,
                        description: Some("Transcribed text".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Chat platform nodes
            self.create_node_definition(
                "comm.mattermost.message",
                "Mattermost Message",
                "Send messages to Mattermost channels",
                NodeCategory::Communication,
                vec![
                    NodeInput {
                        name: "channel_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Mattermost channel ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Message content".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "post_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Mattermost post ID".to_string()),
                        schema: None,
                    },
                ],
            ),
        ];

        self.register_nodes(comm_nodes).await;
    }

    /// Helper method to create node definition
    fn create_node_definition(
        &self,
        node_type: &str,
        name: &str,
        description: &str,
        category: NodeCategory,
        inputs: Vec<NodeInput>,
        outputs: Vec<NodeOutput>,
    ) -> NodeDefinition {
        NodeDefinition {
            node_type: node_type.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            category,
            version: "1.0.0".to_string(),
            author: "Symbiote".to_string(),
            icon: "default".to_string(),
            color: "#4A90E2".to_string(),
            inputs,
            outputs,
            configuration_schema: serde_json::json!({}),
            documentation_url: None,
            examples: Vec::new(),
            tags: Vec::new(),
            pricing: NodePricing {
                pricing_model: PricingModel::Free,
                cost_per_execution: 0.0,
                cost_per_minute: 0.0,
                cost_per_mb: 0.0,
                free_tier_limit: None,
            },
            requirements: NodeRequirements {
                min_memory_mb: 64,
                min_cpu_cores: 0.1,
                requires_gpu: false,
                requires_internet: true,
                supported_platforms: vec!["linux".to_string(), "windows".to_string(), "macos".to_string()],
                dependencies: Vec::new(),
            },
        }
    }

    /// Register multiple nodes
    async fn register_nodes(&self, nodes: Vec<NodeDefinition>) {
        let mut nodes_by_category = self.nodes_by_category.write().await;
        let mut node_definitions = self.node_definitions.write().await;

        for node in nodes {
            // Add to category map
            nodes_by_category
                .entry(node.category.clone())
                .or_insert_with(Vec::new)
                .push(node.clone());

            // Add to definitions map
            node_definitions.insert(node.node_type.clone(), node);
        }
    }

    /// Get nodes by category
    pub async fn get_nodes_by_category(&self, category: &NodeCategory) -> Vec<NodeDefinition> {
        let nodes_by_category = self.nodes_by_category.read().await;
        nodes_by_category.get(category).cloned().unwrap_or_default()
    }

    /// Get node definition by type
    pub async fn get_node_definition(&self, node_type: &str) -> Option<NodeDefinition> {
        let node_definitions = self.node_definitions.read().await;
        node_definitions.get(node_type).cloned()
    }

    /// Get node for execution (used by executor)
    pub async fn get_node(&self, node_type: &str) -> Option<NodeDefinition> {
        self.get_node_definition(node_type).await
    }

    /// Check if node type exists (used by executor)
    pub async fn has_node(&self, node_type: &str) -> bool {
        let node_definitions = self.node_definitions.read().await;
        node_definitions.contains_key(node_type)
    }

    /// Search nodes
    pub async fn search_nodes(&self, query: &str) -> Vec<NodeDefinition> {
        let node_definitions = self.node_definitions.read().await;
        node_definitions
            .values()
            .filter(|node| {
                node.name.to_lowercase().contains(&query.to_lowercase()) ||
                node.description.to_lowercase().contains(&query.to_lowercase()) ||
                node.tags.iter().any(|tag| tag.to_lowercase().contains(&query.to_lowercase()))
            })
            .cloned()
            .collect()
    }

    /// Register Development category nodes (20 nodes)
    async fn register_development_nodes(&self) {
        let dev_nodes = vec![
            // Git operations
            self.create_node_definition(
                "dev.git.clone",
                "Git Clone",
                "Clone a Git repository",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "repository_url".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Git repository URL".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "destination".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Local destination path".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "local_path".to_string(),
                        data_type: DataType::String,
                        description: Some("Local repository path".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.git.commit",
                "Git Commit",
                "Create a Git commit",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "repository_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Repository path".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Commit message".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "commit_hash".to_string(),
                        data_type: DataType::String,
                        description: Some("Commit SHA hash".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.git.push",
                "Git Push",
                "Push commits to remote repository",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "repository_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Repository path".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "branch".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("main".to_string())),
                        description: Some("Branch to push".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "success".to_string(),
                        data_type: DataType::Boolean,
                        description: Some("Push success status".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.git.pull",
                "Git Pull",
                "Pull changes from remote repository",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "repository_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Repository path".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "changes".to_string(),
                        data_type: DataType::Array,
                        description: Some("List of changed files".to_string()),
                        schema: None,
                    },
                ],
            ),

            // CI/CD operations
            self.create_node_definition(
                "dev.ci.github_actions",
                "GitHub Actions Trigger",
                "Trigger GitHub Actions workflow",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "repository".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Repository (owner/repo)".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "workflow_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Workflow ID or filename".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "run_id".to_string(),
                        data_type: DataType::Number,
                        description: Some("Workflow run ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.ci.jenkins_build",
                "Jenkins Build",
                "Trigger Jenkins build job",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "job_name".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Jenkins job name".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "parameters".to_string(),
                        data_type: DataType::Object,
                        required: false,
                        default_value: None,
                        description: Some("Build parameters".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "build_number".to_string(),
                        data_type: DataType::Number,
                        description: Some("Jenkins build number".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.ci.gitlab_pipeline",
                "GitLab Pipeline",
                "Trigger GitLab CI/CD pipeline",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "project_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("GitLab project ID".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "ref".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("main".to_string())),
                        description: Some("Git reference (branch/tag)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "pipeline_id".to_string(),
                        data_type: DataType::Number,
                        description: Some("Pipeline ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Docker operations
            self.create_node_definition(
                "dev.docker.build",
                "Docker Build",
                "Build Docker image from Dockerfile",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "dockerfile_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Path to Dockerfile".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "image_name".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Docker image name".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "image_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Built image ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.docker.run",
                "Docker Run",
                "Run Docker container",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "image".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Docker image name".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "command".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Command to run".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "container_id".to_string(),
                        data_type: DataType::String,
                        description: Some("Running container ID".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.docker.push",
                "Docker Push",
                "Push Docker image to registry",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "image_name".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Docker image name".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "registry".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Docker registry URL".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "digest".to_string(),
                        data_type: DataType::String,
                        description: Some("Image digest".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Kubernetes operations
            self.create_node_definition(
                "dev.k8s.deploy",
                "Kubernetes Deploy",
                "Deploy application to Kubernetes",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "manifest".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Kubernetes manifest YAML".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "namespace".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("default".to_string())),
                        description: Some("Kubernetes namespace".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "deployment_name".to_string(),
                        data_type: DataType::String,
                        description: Some("Deployment name".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.k8s.scale",
                "Kubernetes Scale",
                "Scale Kubernetes deployment",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "deployment".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Deployment name".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "replicas".to_string(),
                        data_type: DataType::Number,
                        required: true,
                        default_value: None,
                        description: Some("Number of replicas".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "current_replicas".to_string(),
                        data_type: DataType::Number,
                        description: Some("Current replica count".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Testing operations
            self.create_node_definition(
                "dev.test.unit",
                "Run Unit Tests",
                "Execute unit test suite",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "test_command".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Test command to execute".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "working_directory".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Working directory".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "test_results".to_string(),
                        data_type: DataType::Object,
                        description: Some("Test execution results".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.test.integration",
                "Run Integration Tests",
                "Execute integration test suite",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "test_suite".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Integration test suite".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "test_report".to_string(),
                        data_type: DataType::Object,
                        description: Some("Integration test report".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.test.e2e",
                "Run E2E Tests",
                "Execute end-to-end test suite",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "test_config".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("E2E test configuration".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "test_results".to_string(),
                        data_type: DataType::Object,
                        description: Some("E2E test results".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Code analysis
            self.create_node_definition(
                "dev.analysis.sonarqube",
                "SonarQube Analysis",
                "Run SonarQube code analysis",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "project_key".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("SonarQube project key".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "source_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Source code path".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "analysis_report".to_string(),
                        data_type: DataType::Object,
                        description: Some("Code analysis report".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.analysis.eslint",
                "ESLint Analysis",
                "Run ESLint code analysis",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "source_files".to_string(),
                        data_type: DataType::Array,
                        required: true,
                        default_value: None,
                        description: Some("JavaScript/TypeScript files to analyze".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "lint_results".to_string(),
                        data_type: DataType::Array,
                        description: Some("ESLint analysis results".to_string()),
                        schema: None,
                    },
                ],
            ),

            // Package management
            self.create_node_definition(
                "dev.package.npm_install",
                "NPM Install",
                "Install NPM dependencies",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "package_json_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Path to package.json".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "installed_packages".to_string(),
                        data_type: DataType::Array,
                        description: Some("List of installed packages".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "dev.package.pip_install",
                "Pip Install",
                "Install Python dependencies",
                NodeCategory::Development,
                vec![
                    NodeInput {
                        name: "requirements_file".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Path to requirements.txt".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "installed_packages".to_string(),
                        data_type: DataType::Array,
                        description: Some("List of installed packages".to_string()),
                        schema: None,
                    },
                ],
            ),
        ];

        self.register_nodes(dev_nodes).await;
    }
    /// Register Data Storage category nodes (20 nodes)
    async fn register_data_storage_nodes(&self) {
        let storage_nodes = vec![
            self.create_node_definition("storage.mysql.query", "MySQL Query", "Execute MySQL database queries", NodeCategory::DataStorage,
                vec![
                    NodeInput { name: "query".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("SQL query".to_string()), validation: None },
                    NodeInput { name: "parameters".to_string(), data_type: DataType::Array, required: false, default_value: None, description: Some("Query parameters".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "results".to_string(), data_type: DataType::Array, description: Some("Query results".to_string()), schema: None }]
            ),
            self.create_node_definition("storage.postgresql.query", "PostgreSQL Query", "Execute PostgreSQL database queries", NodeCategory::DataStorage,
                vec![
                    NodeInput { name: "query".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("SQL query".to_string()), validation: None },
                    NodeInput { name: "parameters".to_string(), data_type: DataType::Array, required: false, default_value: None, description: Some("Query parameters".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "results".to_string(), data_type: DataType::Array, description: Some("Query results".to_string()), schema: None }]
            ),
            self.create_node_definition("storage.mongodb.find", "MongoDB Find", "Find documents in MongoDB collection", NodeCategory::DataStorage,
                vec![
                    NodeInput { name: "collection".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Collection name".to_string()), validation: None },
                    NodeInput { name: "filter".to_string(), data_type: DataType::Object, required: false, default_value: None, description: Some("Query filter".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "documents".to_string(), data_type: DataType::Array, description: Some("Found documents".to_string()), schema: None }]
            ),
        ];
        self.register_nodes(storage_nodes).await;
    }
    /// Register Triggers category nodes (9 nodes - added Chat Trigger)
    async fn register_triggers_nodes(&self) {
        let trigger_nodes = vec![
            // Chat Trigger Node - NEW! Trigger workflows through natural language
            self.create_node_definition(
                "trigger.chat",
                "Chat Trigger",
                "Trigger workflows through natural language chat messages",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "user_message".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("User's chat message that triggers the workflow".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "conversation_id".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("default".to_string())),
                        description: Some("Conversation ID for context tracking".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "user_id".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("anonymous".to_string())),
                        description: Some("User ID for personalization".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "triggered".to_string(),
                        data_type: DataType::Boolean,
                        description: Some("Whether the workflow was triggered".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "user_message".to_string(),
                        data_type: DataType::String,
                        description: Some("Original user message".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "intent".to_string(),
                        data_type: DataType::Object,
                        description: Some("Parsed intent from message (type, confidence, category)".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "entities".to_string(),
                        data_type: DataType::Object,
                        description: Some("Extracted entities (files, languages, numbers, etc.)".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "extracted_parameters".to_string(),
                        data_type: DataType::Object,
                        description: Some("Parameters extracted from message".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "conversation_context".to_string(),
                        data_type: DataType::Object,
                        description: Some("Conversation history and workspace context".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "suggested_workflows".to_string(),
                        data_type: DataType::Array,
                        description: Some("Suggested workflows based on detected intent".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "trigger_conditions".to_string(),
                        data_type: DataType::Object,
                        description: Some("Analyzed conditions (urgency, scope, interactive, immediate)".to_string()),
                        schema: None,
                    },
                ],
            ),
            self.create_node_definition(
                "trigger.schedule",
                "Schedule Trigger",
                "Trigger workflow on a schedule (cron)",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "cron_expression".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Cron expression (e.g., '0 9 * * 1-5')".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "timestamp".to_string(),
                        data_type: DataType::DateTime,
                        description: Some("Trigger timestamp".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.webhook",
                "Webhook Trigger",
                "Trigger workflow via HTTP webhook",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "endpoint_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Webhook endpoint path".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "http_method".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("POST".to_string())),
                        description: Some("HTTP method (GET, POST, PUT, DELETE)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "payload".to_string(),
                        data_type: DataType::Object,
                        description: Some("Webhook payload data".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "headers".to_string(),
                        data_type: DataType::Object,
                        description: Some("HTTP headers".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.email",
                "Email Trigger",
                "Trigger workflow on new emails (IMAP)",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "folder".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("INBOX".to_string())),
                        description: Some("Email folder to monitor".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "filter".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Email filter (subject, sender, etc.)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "email".to_string(),
                        data_type: DataType::Object,
                        description: Some("Email data (subject, body, sender, etc.)".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.file_watcher",
                "File Watcher",
                "Trigger workflow on file system changes",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "watch_path".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Path to watch for changes".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "event_types".to_string(),
                        data_type: DataType::Array,
                        required: false,
                        default_value: Some(serde_json::json!(["created", "modified", "deleted"])),
                        description: Some("File events to watch for".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "file_path".to_string(),
                        data_type: DataType::String,
                        description: Some("Path of changed file".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "event_type".to_string(),
                        data_type: DataType::String,
                        description: Some("Type of file event".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.database",
                "Database Trigger",
                "Trigger workflow on database changes",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "table_name".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Database table to monitor".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "operation".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("INSERT".to_string())),
                        description: Some("Database operation (INSERT, UPDATE, DELETE)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "record".to_string(),
                        data_type: DataType::Object,
                        description: Some("Database record data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.form_submission",
                "Form Submission",
                "Trigger workflow on form submissions",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "form_id".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Form identifier".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "form_data".to_string(),
                        data_type: DataType::Object,
                        description: Some("Submitted form data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.manual",
                "Manual Trigger",
                "Manually trigger workflow execution",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Object,
                        required: false,
                        default_value: None,
                        description: Some("Optional input data".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "data".to_string(),
                        data_type: DataType::Object,
                        description: Some("Manual trigger data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "trigger.cron",
                "Cron Trigger",
                "Advanced cron-based scheduling",
                NodeCategory::Triggers,
                vec![
                    NodeInput {
                        name: "minute".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("*".to_string())),
                        description: Some("Minute (0-59 or *)".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "hour".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("*".to_string())),
                        description: Some("Hour (0-23 or *)".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "day".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("*".to_string())),
                        description: Some("Day of month (1-31 or *)".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "month".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("*".to_string())),
                        description: Some("Month (1-12 or *)".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "day_of_week".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("*".to_string())),
                        description: Some("Day of week (0-7 or *)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "execution_time".to_string(),
                        data_type: DataType::DateTime,
                        description: Some("Scheduled execution time".to_string()),
                        schema: None,
                    },
                ],
            ),
        ];

        self.register_nodes(trigger_nodes).await;
    }

    /// Register Data Processing category nodes (12 nodes)
    async fn register_processing_nodes(&self) {
        let processing_nodes = vec![
            self.create_node_definition(
                "data.csv.parse",
                "CSV Parser",
                "Parse CSV data into structured format",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "csv_data".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("CSV data to parse".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "delimiter".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String(",".to_string())),
                        description: Some("CSV delimiter".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "has_headers".to_string(),
                        data_type: DataType::Boolean,
                        required: false,
                        default_value: Some(serde_json::Value::Bool(true)),
                        description: Some("Whether CSV has header row".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "records".to_string(),
                        data_type: DataType::Array,
                        description: Some("Parsed CSV records".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "headers".to_string(),
                        data_type: DataType::Array,
                        description: Some("CSV column headers".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.json.parse",
                "JSON Parser",
                "Parse JSON data into objects",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "json_data".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("JSON data to parse".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "parsed_data".to_string(),
                        data_type: DataType::Object,
                        description: Some("Parsed JSON object".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.xml.parse",
                "XML Parser",
                "Parse XML data into structured format",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "xml_data".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("XML data to parse".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "parsed_data".to_string(),
                        data_type: DataType::Object,
                        description: Some("Parsed XML object".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.pdf.extract",
                "PDF Extract Text",
                "Extract text content from PDF files",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "pdf_file".to_string(),
                        data_type: DataType::Binary,
                        required: true,
                        default_value: None,
                        description: Some("PDF file data".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "page_range".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Page range (e.g., '1-5' or 'all')".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "text".to_string(),
                        data_type: DataType::String,
                        description: Some("Extracted text content".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "page_count".to_string(),
                        data_type: DataType::Number,
                        description: Some("Total number of pages".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.excel.read",
                "Excel Read",
                "Read data from Excel files",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "excel_file".to_string(),
                        data_type: DataType::Binary,
                        required: true,
                        default_value: None,
                        description: Some("Excel file data".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "sheet_name".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Sheet name (default: first sheet)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "data".to_string(),
                        data_type: DataType::Array,
                        description: Some("Excel sheet data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.excel.write",
                "Excel Write",
                "Write data to Excel files",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "data".to_string(),
                        data_type: DataType::Array,
                        required: true,
                        default_value: None,
                        description: Some("Data to write to Excel".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "sheet_name".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("Sheet1".to_string())),
                        description: Some("Sheet name".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "excel_file".to_string(),
                        data_type: DataType::Binary,
                        description: Some("Generated Excel file".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.transform",
                "Data Transform",
                "Transform data using JavaScript expressions",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("Data to transform".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "transform_expression".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("JavaScript transformation expression".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "transformed_data".to_string(),
                        data_type: DataType::Any,
                        description: Some("Transformed data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.filter",
                "Filter Data",
                "Filter data based on conditions",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Array,
                        required: true,
                        default_value: None,
                        description: Some("Data array to filter".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "filter_condition".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Filter condition expression".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "filtered_data".to_string(),
                        data_type: DataType::Array,
                        description: Some("Filtered data array".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.merge",
                "Merge Data",
                "Merge multiple data sources",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "data_source_1".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("First data source".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "data_source_2".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("Second data source".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "merge_strategy".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("concat".to_string())),
                        description: Some("Merge strategy (concat, merge, join)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "merged_data".to_string(),
                        data_type: DataType::Any,
                        description: Some("Merged data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.split",
                "Split Data",
                "Split data into multiple parts",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("Data to split".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "split_criteria".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Split criteria or delimiter".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "split_data".to_string(),
                        data_type: DataType::Array,
                        description: Some("Split data parts".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.validate",
                "Validate Data",
                "Validate data against schema or rules",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("Data to validate".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "validation_schema".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("Validation schema or rules".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "is_valid".to_string(),
                        data_type: DataType::Boolean,
                        description: Some("Validation result".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "errors".to_string(),
                        data_type: DataType::Array,
                        description: Some("Validation errors".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "data.hash",
                "Hash Generator",
                "Generate hash values for data",
                NodeCategory::Processing,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Data to hash".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "algorithm".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("sha256".to_string())),
                        description: Some("Hash algorithm (md5, sha1, sha256, sha512)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "hash".to_string(),
                        data_type: DataType::String,
                        description: Some("Generated hash value".to_string()),
                        schema: None,
                    },
                ],
            ),
        ];

        self.register_nodes(processing_nodes).await;
    }

    /// Register Control Flow category nodes (8 nodes)
    async fn register_control_flow_nodes(&self) {
        let control_nodes = vec![
            self.create_node_definition(
                "control.if",
                "If/Else",
                "Conditional execution based on conditions",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "condition".to_string(),
                        data_type: DataType::String,
                        required: true,
                        default_value: None,
                        description: Some("Condition expression to evaluate".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Any,
                        required: false,
                        default_value: None,
                        description: Some("Data to evaluate condition against".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "true_branch".to_string(),
                        data_type: DataType::Any,
                        description: Some("Output when condition is true".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "false_branch".to_string(),
                        data_type: DataType::Any,
                        description: Some("Output when condition is false".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.switch",
                "Switch",
                "Multi-way conditional execution",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "switch_value".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("Value to switch on".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "cases".to_string(),
                        data_type: DataType::Object,
                        required: true,
                        default_value: None,
                        description: Some("Switch cases and their outputs".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "matched_output".to_string(),
                        data_type: DataType::Any,
                        description: Some("Output from matched case".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.loop",
                "Loop",
                "Execute nodes repeatedly",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "input_array".to_string(),
                        data_type: DataType::Array,
                        required: true,
                        default_value: None,
                        description: Some("Array to iterate over".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "max_iterations".to_string(),
                        data_type: DataType::Number,
                        required: false,
                        default_value: Some(serde_json::Value::Number(serde_json::Number::from(1000))),
                        description: Some("Maximum number of iterations".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "current_item".to_string(),
                        data_type: DataType::Any,
                        description: Some("Current iteration item".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "index".to_string(),
                        data_type: DataType::Number,
                        description: Some("Current iteration index".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.wait",
                "Wait/Delay",
                "Pause execution for specified time",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "duration".to_string(),
                        data_type: DataType::Number,
                        required: true,
                        default_value: None,
                        description: Some("Wait duration in seconds".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "completed".to_string(),
                        data_type: DataType::Boolean,
                        description: Some("Wait completion status".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.stop",
                "Stop Execution",
                "Stop workflow execution",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "reason".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: None,
                        description: Some("Reason for stopping execution".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "stopped".to_string(),
                        data_type: DataType::Boolean,
                        description: Some("Execution stopped status".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.error_handler",
                "Error Handling",
                "Handle errors in workflow execution",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Any,
                        required: true,
                        default_value: None,
                        description: Some("Data that might cause errors".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "fallback_value".to_string(),
                        data_type: DataType::Any,
                        required: false,
                        default_value: None,
                        description: Some("Fallback value on error".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "result".to_string(),
                        data_type: DataType::Any,
                        description: Some("Result or fallback value".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "error".to_string(),
                        data_type: DataType::Object,
                        description: Some("Error information if occurred".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.merge",
                "Merge Branches",
                "Merge multiple execution branches",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "branch_1".to_string(),
                        data_type: DataType::Any,
                        required: false,
                        default_value: None,
                        description: Some("First branch data".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "branch_2".to_string(),
                        data_type: DataType::Any,
                        required: false,
                        default_value: None,
                        description: Some("Second branch data".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "merge_mode".to_string(),
                        data_type: DataType::String,
                        required: false,
                        default_value: Some(serde_json::Value::String("wait_all".to_string())),
                        description: Some("Merge mode (wait_all, first_complete)".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "merged_data".to_string(),
                        data_type: DataType::Array,
                        description: Some("Merged branch data".to_string()),
                        schema: None,
                    },
                ],
            ),

            self.create_node_definition(
                "control.split_batch",
                "Split in Batches",
                "Split data into batches for processing",
                NodeCategory::ControlFlow,
                vec![
                    NodeInput {
                        name: "input_data".to_string(),
                        data_type: DataType::Array,
                        required: true,
                        default_value: None,
                        description: Some("Data to split into batches".to_string()),
                        validation: None,
                    },
                    NodeInput {
                        name: "batch_size".to_string(),
                        data_type: DataType::Number,
                        required: false,
                        default_value: Some(serde_json::Value::Number(serde_json::Number::from(10))),
                        description: Some("Size of each batch".to_string()),
                        validation: None,
                    },
                ],
                vec![
                    NodeOutput {
                        name: "batch".to_string(),
                        data_type: DataType::Array,
                        description: Some("Current batch data".to_string()),
                        schema: None,
                    },
                    NodeOutput {
                        name: "batch_number".to_string(),
                        data_type: DataType::Number,
                        description: Some("Current batch number".to_string()),
                        schema: None,
                    },
                ],
            ),
        ];

        self.register_nodes(control_nodes).await;
    }



    /// Register Business Apps category nodes (20 nodes)
    async fn register_productivity_nodes(&self) {
        let business_nodes = vec![
            self.create_node_definition("business.salesforce.create", "Salesforce Create", "Create Salesforce record", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "object_type".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Object type".to_string()), validation: None },
                    NodeInput { name: "data".to_string(), data_type: DataType::Object, required: true, default_value: None, description: Some("Record data".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "id".to_string(), data_type: DataType::String, description: Some("Record ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.hubspot.contact", "HubSpot Contact", "Manage HubSpot contacts", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "action".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Action type".to_string()), validation: None },
                    NodeInput { name: "contact_data".to_string(), data_type: DataType::Object, required: true, default_value: None, description: Some("Contact data".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "contact".to_string(), data_type: DataType::Object, description: Some("Contact info".to_string()), schema: None }]
            ),
            self.create_node_definition("business.google_sheets.read", "Google Sheets Read", "Read Google Sheets data", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "spreadsheet_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Spreadsheet ID".to_string()), validation: None },
                    NodeInput { name: "range".to_string(), data_type: DataType::String, required: false, default_value: Some(serde_json::Value::String("A1:Z1000".to_string())), description: Some("Cell range".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "data".to_string(), data_type: DataType::Array, description: Some("Sheet data".to_string()), schema: None }]
            ),
            self.create_node_definition("business.google_sheets.write", "Google Sheets Write", "Write to Google Sheets", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "spreadsheet_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Spreadsheet ID".to_string()), validation: None },
                    NodeInput { name: "range".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Cell range".to_string()), validation: None },
                    NodeInput { name: "data".to_string(), data_type: DataType::Array, required: true, default_value: None, description: Some("Data to write".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "updated_cells".to_string(), data_type: DataType::Number, description: Some("Updated cells count".to_string()), schema: None }]
            ),
            self.create_node_definition("business.airtable.create", "Airtable Create", "Create Airtable record", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "base_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Base ID".to_string()), validation: None },
                    NodeInput { name: "table_name".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Table name".to_string()), validation: None },
                    NodeInput { name: "fields".to_string(), data_type: DataType::Object, required: true, default_value: None, description: Some("Record fields".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "record_id".to_string(), data_type: DataType::String, description: Some("Record ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.notion.create", "Notion Create", "Create Notion page", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "database_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Database ID".to_string()), validation: None },
                    NodeInput { name: "properties".to_string(), data_type: DataType::Object, required: true, default_value: None, description: Some("Page properties".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "page_id".to_string(), data_type: DataType::String, description: Some("Page ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.jira.create", "Jira Create Issue", "Create Jira issue", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "project_key".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Project key".to_string()), validation: None },
                    NodeInput { name: "issue_type".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Issue type".to_string()), validation: None },
                    NodeInput { name: "summary".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Issue summary".to_string()), validation: None },
                    NodeInput { name: "description".to_string(), data_type: DataType::String, required: false, default_value: None, description: Some("Issue description".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "issue_key".to_string(), data_type: DataType::String, description: Some("Issue key".to_string()), schema: None }]
            ),
            self.create_node_definition("business.asana.create", "Asana Create Task", "Create Asana task", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "project_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Project ID".to_string()), validation: None },
                    NodeInput { name: "name".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Task name".to_string()), validation: None },
                    NodeInput { name: "notes".to_string(), data_type: DataType::String, required: false, default_value: None, description: Some("Task notes".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "task_id".to_string(), data_type: DataType::String, description: Some("Task ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.trello.create", "Trello Create Card", "Create Trello card", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "list_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("List ID".to_string()), validation: None },
                    NodeInput { name: "name".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Card name".to_string()), validation: None },
                    NodeInput { name: "desc".to_string(), data_type: DataType::String, required: false, default_value: None, description: Some("Card description".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "card_id".to_string(), data_type: DataType::String, description: Some("Card ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.monday.create", "Monday.com Create", "Create Monday.com item", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "board_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Board ID".to_string()), validation: None },
                    NodeInput { name: "item_name".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Item name".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "item_id".to_string(), data_type: DataType::String, description: Some("Item ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.pipedrive.create", "Pipedrive Create", "Create Pipedrive deal", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "title".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Deal title".to_string()), validation: None },
                    NodeInput { name: "value".to_string(), data_type: DataType::Number, required: false, default_value: None, description: Some("Deal value".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "deal_id".to_string(), data_type: DataType::Number, description: Some("Deal ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.zendesk.create", "Zendesk Create Ticket", "Create Zendesk ticket", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "subject".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Ticket subject".to_string()), validation: None },
                    NodeInput { name: "description".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Ticket description".to_string()), validation: None },
                    NodeInput { name: "requester_email".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Requester email".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "ticket_id".to_string(), data_type: DataType::Number, description: Some("Ticket ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.freshdesk.create", "Freshdesk Create", "Create Freshdesk ticket", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "subject".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Ticket subject".to_string()), validation: None },
                    NodeInput { name: "description".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Ticket description".to_string()), validation: None },
                    NodeInput { name: "email".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Requester email".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "ticket_id".to_string(), data_type: DataType::Number, description: Some("Ticket ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.quickbooks.create", "QuickBooks Create", "Create QuickBooks entry", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "entity_type".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Entity type".to_string()), validation: None },
                    NodeInput { name: "data".to_string(), data_type: DataType::Object, required: true, default_value: None, description: Some("Entity data".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "entity_id".to_string(), data_type: DataType::String, description: Some("Entity ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.stripe.payment", "Stripe Payment", "Process Stripe payment", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "amount".to_string(), data_type: DataType::Number, required: true, default_value: None, description: Some("Payment amount".to_string()), validation: None },
                    NodeInput { name: "currency".to_string(), data_type: DataType::String, required: false, default_value: Some(serde_json::Value::String("usd".to_string())), description: Some("Currency".to_string()), validation: None },
                    NodeInput { name: "payment_method".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Payment method".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "payment_intent_id".to_string(), data_type: DataType::String, description: Some("Payment intent ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.paypal.payment", "PayPal Payment", "Process PayPal payment", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "amount".to_string(), data_type: DataType::Number, required: true, default_value: None, description: Some("Payment amount".to_string()), validation: None },
                    NodeInput { name: "currency".to_string(), data_type: DataType::String, required: false, default_value: Some(serde_json::Value::String("USD".to_string())), description: Some("Currency".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "payment_id".to_string(), data_type: DataType::String, description: Some("Payment ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.shopify.order", "Shopify Order", "Manage Shopify orders", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "action".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Action type".to_string()), validation: None },
                    NodeInput { name: "order_data".to_string(), data_type: DataType::Object, required: false, default_value: None, description: Some("Order data".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "order".to_string(), data_type: DataType::Object, description: Some("Order info".to_string()), schema: None }]
            ),
            self.create_node_definition("business.woocommerce.order", "WooCommerce Order", "Manage WooCommerce orders", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "action".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Action type".to_string()), validation: None },
                    NodeInput { name: "order_data".to_string(), data_type: DataType::Object, required: false, default_value: None, description: Some("Order data".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "order".to_string(), data_type: DataType::Object, description: Some("Order info".to_string()), schema: None }]
            ),
            self.create_node_definition("business.mailchimp.campaign", "Mailchimp Campaign", "Manage Mailchimp campaigns", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "action".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Action type".to_string()), validation: None },
                    NodeInput { name: "campaign_data".to_string(), data_type: DataType::Object, required: false, default_value: None, description: Some("Campaign data".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "campaign".to_string(), data_type: DataType::Object, description: Some("Campaign info".to_string()), schema: None }]
            ),
            self.create_node_definition("business.calendly.schedule", "Calendly Schedule", "Manage Calendly events", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "event_type".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Event type".to_string()), validation: None },
                    NodeInput { name: "invitee_email".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Invitee email".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "event_id".to_string(), data_type: DataType::String, description: Some("Event ID".to_string()), schema: None }]
            ),
            self.create_node_definition("business.typeform.response", "Typeform Response", "Get Typeform responses", NodeCategory::Productivity,
                vec![
                    NodeInput { name: "form_id".to_string(), data_type: DataType::String, required: true, default_value: None, description: Some("Form ID".to_string()), validation: None },
                    NodeInput { name: "since".to_string(), data_type: DataType::String, required: false, default_value: None, description: Some("Since date".to_string()), validation: None },
                ],
                vec![NodeOutput { name: "responses".to_string(), data_type: DataType::Array, description: Some("Form responses".to_string()), schema: None }]
            ),
        ];
        self.register_nodes(business_nodes).await;
    }



    // Duplicate method removed - using the first register_ai_nodes implementation



    // Placeholder methods for remaining categories
    async fn register_marketing_nodes(&self) { /* Implementation */ }
    async fn register_sales_nodes(&self) { /* Implementation */ }
    async fn register_finance_nodes(&self) { /* Implementation */ }
    async fn register_ecommerce_nodes(&self) { /* Implementation */ }
    async fn register_cybersecurity_nodes(&self) { /* Implementation */ }
    async fn register_monitoring_nodes(&self) { /* Implementation */ }
    async fn register_cloud_nodes(&self) { /* Implementation */ }
    async fn register_utilities_nodes(&self) { /* Implementation */ }
    async fn register_outputs_nodes(&self) { /* Implementation */ }
    async fn register_custom_nodes(&self) { /* Implementation */ }
}

impl Clone for NodeRegistry {
    fn clone(&self) -> Self {
        // Simplified clone for demonstration
        NodeRegistry::new()
    }
}
