//! # SYMBIOTE Personal AI Assistant
//! 
//! The master AI orchestrator for Symbiote IDE - an all-purpose AI agent chat
//! that can intelligently coordinate specialized agents to accomplish ANY task.
//! 
//! SYMBIOTE is the central brain that understands user requests and orchestrates
//! the right combination of agents, tools, and systems to get things done.
//! 
//! Following Week 17-18 Personal AI Assistant implementation plan.

pub mod core;
pub mod agents;
pub mod orchestrator;
pub mod monitor;
pub mod context_integrator;
pub mod task_decomposer;
pub mod nlp;
pub mod synthesis;

// Re-export main types
pub use core::*;
pub use orchestrator::*;
pub use monitor::*;
pub use context_integrator::*;
pub use task_decomposer::*;
pub use nlp::*;
pub use synthesis::*;

// Re-export agent implementations
pub use agents::*;

use crate::{Result, SymbioteError};
use crate::context::ContextBus;
use crate::memory::{MemorySystem, AgentRuleManager};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

/// SYMBIOTE - The Personal AI Assistant
/// 
/// Master orchestrator that can intelligently coordinate specialized agents
/// to accomplish any task the user requests. Context-aware of all Symbiote
/// systems and user activity across panels.
#[derive(Debug)]
pub struct Symbiote {
    /// Core orchestration engine
    orchestrator: AgentOrchestrator,
    
    /// Registry of all available specialized agents
    agent_registry: AgentRegistry,
    
    /// Monitor for tracking running agents
    agent_monitor: AgentMonitor,
    
    /// Deep integration with Symbiote's context systems
    context_integrator: Arc<RwLock<ContextIntegrator>>,
    
    /// Natural language processing for understanding user intent
    nlp_engine: NaturalLanguageProcessor,
    
    /// Task decomposition for breaking down complex requests
    task_decomposer: TaskDecomposer,
    
    /// Result synthesis for combining agent outputs
    result_synthesizer: ResultSynthesizer,

    /// Memory system for persistent memory and learning
    memory_system: MemorySystem,

    /// Current conversation context
    conversation_context: Arc<RwLock<ConversationContext>>,

    /// Integration with Symbiote's context bus
    context_bus: Option<Arc<ContextBus>>,
}

impl Symbiote {
    /// Create new SYMBIOTE Personal AI Assistant
    pub fn new() -> Self {
        let agent_registry = AgentRegistry::new();
        let agent_monitor = AgentMonitor::new();
        let context_integrator = Arc::new(RwLock::new(ContextIntegrator::new()));
        let nlp_engine = NaturalLanguageProcessor::new();
        let task_decomposer = TaskDecomposer::new();
        let result_synthesizer = ResultSynthesizer::new();
        
        Self {
            orchestrator: AgentOrchestrator::new(agent_registry.clone(), agent_monitor.clone()),
            agent_registry,
            agent_monitor,
            context_integrator,
            nlp_engine,
            task_decomposer,
            result_synthesizer,
            memory_system: MemorySystem::new(),
            conversation_context: Arc::new(RwLock::new(ConversationContext::new())),
            context_bus: None,
        }
    }

    /// Create SYMBIOTE with context bus integration
    pub fn with_context_bus(context_bus: Arc<ContextBus>) -> Self {
        let mut symbiote = Self::new();
        symbiote.context_bus = Some(context_bus.clone());
        // Note: context_integrator is wrapped in Arc<RwLock>, would need async context to set
        // For now, we'll handle this in the initialization
        symbiote.memory_system = MemorySystem::with_context_bus(context_bus);
        symbiote
    }

    /// Process user message and orchestrate appropriate agents
    pub async fn process_message(&self, message: UserMessage) -> Result<SymbioteResponse> {
        // Update conversation context
        {
            let mut context = self.conversation_context.write().await;
            context.add_user_message(message.clone());
        }

        // Get current Symbiote context (what panel user is on, etc.)
        let integrator = self.context_integrator.read().await;
        let symbiote_context = integrator.get_current_context().await?;

        // Process natural language to understand intent
        let intent = self.nlp_engine.process_message(&message, &symbiote_context).await?;

        // Decompose into actionable tasks
        let tasks = self.task_decomposer.decompose_intent(&intent, &symbiote_context).await?;

        // Orchestrate agents to execute tasks
        let execution_plan = self.orchestrator.create_execution_plan(&tasks).await?;
        let agent_results = self.orchestrator.execute_plan(execution_plan).await?;

        // Synthesize results into coherent response
        let response = self.result_synthesizer.synthesize_results(
            &agent_results,
            &intent,
            &symbiote_context,
        ).await?;

        // Update conversation context with response
        {
            let mut context = self.conversation_context.write().await;
            context.add_symbiote_response(response.clone());
        }

        Ok(response)
    }

    /// Get list of currently running agents
    pub async fn get_running_agents(&self) -> Vec<RunningAgent> {
        self.agent_monitor.get_running_agents().await
    }

    /// Get details for a specific agent
    pub async fn get_agent_details(&self, agent_id: &str) -> Result<AgentDetails> {
        self.agent_monitor.get_agent_details(agent_id).await
    }

    /// Cancel a running agent
    pub async fn cancel_agent(&self, agent_id: &str) -> Result<()> {
        self.agent_monitor.cancel_agent(agent_id).await
    }

    /// Get conversation history
    pub async fn get_conversation_history(&self) -> ConversationHistory {
        let context = self.conversation_context.read().await;
        context.get_history()
    }

    /// Clear conversation context
    pub async fn clear_conversation(&self) -> Result<()> {
        let mut context = self.conversation_context.write().await;
        context.clear();
        Ok(())
    }

    /// Register a new specialized agent
    pub async fn register_agent(&self, agent: Box<dyn SpecializedAgent>) -> Result<()> {
        self.agent_registry.register_agent(agent).await
    }

    /// Get available agent capabilities
    pub async fn get_agent_capabilities(&self) -> Vec<AgentCapability> {
        self.agent_registry.get_capabilities().await
    }

    /// Update current panel context (when user switches panels)
    pub async fn update_panel_context(&self, panel_info: PanelInfo) -> Result<()> {
        let mut integrator = self.context_integrator.write().await;
        integrator.update_panel_context(panel_info).await
    }

    /// Get current Symbiote system status
    pub async fn get_system_status(&self) -> SymbioteSystemStatus {
        SymbioteSystemStatus {
            running_agents: self.agent_monitor.get_agent_count().await,
            active_workflows: {
                let integrator = self.context_integrator.read().await;
                integrator.get_active_workflow_count().await
            },
            open_notebooks: {
                let integrator = self.context_integrator.read().await;
                integrator.get_open_notebook_count().await
            },
            current_panel: {
                let integrator = self.context_integrator.read().await;
                integrator.get_current_panel().await
            },
            workspace_context: {
                let integrator = self.context_integrator.read().await;
                integrator.get_workspace_summary().await
            },
        }
    }

    /// Get agent rules for user
    pub async fn get_agent_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str) -> Result<crate::memory::AgentRules> {
        self.memory_system.get_agent_rules(workspace_id, agent_type, user_id).await
    }

    /// Update agent rules for user
    pub async fn update_agent_rules(&self, workspace_id: &str, agent_type: &str, user_id: &str, rules: crate::memory::AgentRules) -> Result<()> {
        self.memory_system.update_agent_rules(workspace_id, agent_type, user_id, rules).await
    }

    /// Get enhanced system prompt for agent with user rules applied
    pub async fn get_agent_system_prompt(&self, workspace_id: &str, agent_type: &str, user_id: &str, base_prompt: &str) -> Result<String> {
        self.memory_system.get_agent_system_prompt(workspace_id, agent_type, user_id, base_prompt).await
    }

    /// Store user memory
    pub async fn store_memory(&self, memory: crate::memory::Memory) -> Result<String> {
        self.memory_system.store_memory(memory).await
    }

    /// Search user memories
    pub async fn search_memories(&self, query: &str, user_id: &str, limit: usize) -> Result<Vec<crate::memory::Memory>> {
        self.memory_system.semantic_search(query, user_id, limit).await
    }

    /// Get user memory profile
    pub async fn get_user_memory(&self, user_id: &str) -> Result<crate::memory::UserMemory> {
        self.memory_system.get_user_memory(user_id).await
    }

    /// Learn from user interaction
    pub async fn learn_from_interaction(&self, interaction: crate::memory::UserInteraction) -> Result<()> {
        self.memory_system.learn_from_interaction(interaction).await
    }

    /// Shutdown SYMBIOTE and all running agents
    pub async fn shutdown(&self) -> Result<()> {
        // Cancel all running agents
        self.agent_monitor.cancel_all_agents().await?;

        // Shutdown orchestrator
        self.orchestrator.shutdown().await?;

        Ok(())
    }
}

/// User message to SYMBIOTE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub panel_context: Option<PanelInfo>,
    pub attachments: Vec<MessageAttachment>,
}

/// SYMBIOTE's response to user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteResponse {
    pub id: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub agent_actions: Vec<AgentAction>,
    pub results: Vec<ActionResult>,
    pub suggestions: Vec<Suggestion>,
    pub status: ResponseStatus,
}

/// Information about current panel/context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelInfo {
    pub panel_type: PanelType,
    pub panel_id: String,
    pub active_file: Option<String>,
    pub cursor_position: Option<CursorPosition>,
    pub selection: Option<TextSelection>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of panels in Symbiote IDE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelType {
    Editor,
    Notebook,
    Workflow,
    Terminal,
    FileExplorer,
    CryptoTrading,
    Settings,
    Chat,
    Other(String),
}

/// Current system status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbioteSystemStatus {
    pub running_agents: usize,
    pub active_workflows: usize,
    pub open_notebooks: usize,
    pub current_panel: Option<PanelInfo>,
    pub workspace_context: WorkspaceSummary,
}

/// Workspace summary for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub project_name: Option<String>,
    pub open_files: Vec<String>,
    pub recent_activity: Vec<String>,
    pub git_status: Option<GitStatus>,
}

/// Git status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub modified_files: Vec<String>,
    pub staged_files: Vec<String>,
    pub commits_ahead: usize,
    pub commits_behind: usize,
}

impl UserMessage {
    /// Create new user message
    pub fn new(content: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            timestamp: Utc::now(),
            user_id: None,
            panel_context: None,
            attachments: Vec::new(),
        }
    }

    /// Create message with panel context
    pub fn with_panel_context(content: String, panel_info: PanelInfo) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content,
            timestamp: Utc::now(),
            user_id: None,
            panel_context: Some(panel_info),
            attachments: Vec::new(),
        }
    }
}

impl Default for Symbiote {
    fn default() -> Self {
        Self::new()
    }
}
