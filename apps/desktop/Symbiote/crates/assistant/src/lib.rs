//! # Symbiote Assistant
//!
//! Personal AI Assistant that serves as the central intelligence and unified interface for Symbiote.
//! Features:
//! - Master orchestrator that intelligently routes tasks to appropriate agents and systems
//! - Unified chat interface for all Symbiote interactions
//! - Context awareness of workspace, panel, and user activity
//! - Multi-modal interaction support (text, voice, image, files)
//! - Personalization and adaptive behavior learning
//! - Cross-panel agency for context-aware assistance
//! - Desktop automation and real-life assistant tools

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod orchestrator;
pub mod chat;
pub mod context;
pub mod personalization;
pub mod multimodal;
pub mod agents;
pub mod tools;
pub mod productivity;
pub mod desktop;
pub mod ui;
pub mod server;
pub mod types;

// Re-export main types
pub use orchestrator::{MasterOrchestrator, RequestRouter, RoutingDecision};
pub use chat::{ChatInterface, UserMessage, AssistantResponse, ResponseStream};
pub use context::{ContextManager, UserContext, WorkspaceInfo, CrossPanelAgency};
pub use personalization::{PersonalizationEngine, UserPreferences, Suggestion};
pub use multimodal::{MultiModalProcessor, AudioInput, ImageInput, FileInput};
pub use agents::{AgentCoordinator, AgentSelection};
pub use tools::{ToolManager, ToolExecution};
pub use productivity::{UserProductivitySuite, TaskManager, CalendarScheduling};
pub use desktop::{DesktopController, DesktopAutomation};
pub use ui::{AssistantUI, ChatWindow, ContextPanel};
pub use server::{AssistantServer, WebSocketHandler, ApiRouter};
pub use types::{AssistantConfig, AssistantError, AssistantResult};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use symbiote_ai::AIProviderManager;
use symbiote_agents::AgentFramework;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The main Personal Assistant that coordinates all Symbiote interactions
pub struct PersonalAssistant {
    /// Master orchestrator for intelligent routing
    pub orchestrator: Arc<MasterOrchestrator>,

    /// Chat interface for user interactions
    pub chat_interface: Arc<ChatInterface>,

    /// Context management and awareness
    pub context_manager: Arc<ContextManager>,

    /// Personalization and learning engine
    pub personalization_engine: Arc<PersonalizationEngine>,

    /// Multi-modal processing capabilities
    pub multimodal_processor: Arc<MultiModalProcessor>,

    /// Agent coordination and management
    pub agent_coordinator: Arc<AgentCoordinator>,

    /// Tool management and execution
    pub tool_manager: Arc<ToolManager>,

    /// User productivity suite
    pub productivity_suite: Arc<UserProductivitySuite>,

    /// Desktop automation controller
    pub desktop_controller: Arc<DesktopController>,

    /// Cross-panel agency system
    pub cross_panel_agency: Arc<CrossPanelAgency>,

    /// Assistant UI system
    pub ui_system: Arc<AssistantUI>,

    /// AI provider integration
    pub ai_provider: Arc<AIProviderManager>,

    /// Agent framework integration
    pub agent_framework: Arc<AgentFramework>,

    /// Configuration
    config: Arc<RwLock<AssistantConfig>>,
}

/// Configuration for the assistant system
#[derive(Debug, Clone)]
pub struct AssistantConfig {
    /// Enable AI-powered features
    pub enable_ai_features: bool,

    /// Enable voice interaction
    pub enable_voice: bool,

    /// Enable vision capabilities
    pub enable_vision: bool,

    /// Enable desktop automation
    pub enable_desktop_automation: bool,

    /// Database connection string
    pub database_url: String,

    /// Server configuration
    pub server: ServerConfig,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server host
    pub host: String,

    /// Server port
    pub port: u16,

    /// Enable WebSocket support
    pub enable_websocket: bool,
}

impl Default for AssistantConfig {
    fn default() -> Self {
        Self {
            enable_ai_features: true,
            enable_voice: true,
            enable_vision: true,
            enable_desktop_automation: false, // Disabled by default for security
            database_url: "postgresql://localhost/symbiote_assistant".to_string(),
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8083,
                enable_websocket: true,
            },
        }
    }
}

impl PersonalAssistant {
    /// Create a new personal assistant instance
    pub async fn new(
        config: AssistantConfig,
        ai_provider: Arc<AIProviderManager>,
        agent_framework: Arc<AgentFramework>,
    ) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));

        // Initialize core components
        let orchestrator = Arc::new(MasterOrchestrator::new(config.clone(), ai_provider.clone()).await?);
        let chat_interface = Arc::new(ChatInterface::new(config.clone()).await?);
        let context_manager = Arc::new(ContextManager::new(config.clone()).await?);
        let personalization_engine = Arc::new(PersonalizationEngine::new(config.clone()).await?);
        let multimodal_processor = Arc::new(MultiModalProcessor::new(config.clone()).await?);
        let agent_coordinator = Arc::new(AgentCoordinator::new(config.clone(), agent_framework.clone()).await?);
        let tool_manager = Arc::new(ToolManager::new(config.clone()).await?);
        let productivity_suite = Arc::new(UserProductivitySuite::new(config.clone()).await?);
        let desktop_controller = Arc::new(DesktopController::new(config.clone()).await?);
        let cross_panel_agency = Arc::new(CrossPanelAgency::new(config.clone()).await?);
        let ui_system = Arc::new(AssistantUI::new(config.clone()).await?);

        Ok(Self {
            orchestrator,
            chat_interface,
            context_manager,
            personalization_engine,
            multimodal_processor,
            agent_coordinator,
            tool_manager,
            productivity_suite,
            desktop_controller,
            cross_panel_agency,
            ui_system,
            ai_provider,
            agent_framework,
            config,
        })
    }

    /// Start the personal assistant
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote Personal Assistant");

        // Initialize all components
        self.orchestrator.initialize().await?;
        self.chat_interface.initialize().await?;
        self.context_manager.initialize().await?;
        self.personalization_engine.initialize().await?;
        self.multimodal_processor.initialize().await?;
        self.agent_coordinator.initialize().await?;
        self.tool_manager.initialize().await?;
        self.productivity_suite.initialize().await?;
        self.desktop_controller.initialize().await?;
        self.cross_panel_agency.initialize().await?;
        self.ui_system.initialize().await?;

        tracing::info!("Symbiote Personal Assistant started successfully");
        Ok(())
    }

    /// Stop the personal assistant
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote Personal Assistant");

        // Shutdown all components in reverse order
        self.ui_system.shutdown().await?;
        self.cross_panel_agency.shutdown().await?;
        self.desktop_controller.shutdown().await?;
        self.productivity_suite.shutdown().await?;
        self.tool_manager.shutdown().await?;
        self.agent_coordinator.shutdown().await?;
        self.multimodal_processor.shutdown().await?;
        self.personalization_engine.shutdown().await?;
        self.context_manager.shutdown().await?;
        self.chat_interface.shutdown().await?;
        self.orchestrator.shutdown().await?;

        tracing::info!("Symbiote Personal Assistant stopped successfully");
        Ok(())
    }

    /// Process a user message and generate a response
    pub async fn process_message(&self, message: UserMessage) -> SymbioteResult<AssistantResponse> {
        self.orchestrator.process_message(message).await
    }

    /// Process a user message with streaming response
    pub async fn process_message_stream(&self, message: UserMessage) -> SymbioteResult<ResponseStream> {
        self.orchestrator.process_message_stream(message).await
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> AssistantConfig {
        self.config.read().await.clone()
    }

    /// Update the configuration
    pub async fn update_config(&self, new_config: AssistantConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;

        // Notify all components of config change
        self.orchestrator.on_config_changed(&*config).await?;
        self.chat_interface.on_config_changed(&*config).await?;
        self.context_manager.on_config_changed(&*config).await?;
        self.personalization_engine.on_config_changed(&*config).await?;
        self.multimodal_processor.on_config_changed(&*config).await?;
        self.agent_coordinator.on_config_changed(&*config).await?;
        self.tool_manager.on_config_changed(&*config).await?;
        self.productivity_suite.on_config_changed(&*config).await?;
        self.desktop_controller.on_config_changed(&*config).await?;
        self.cross_panel_agency.on_config_changed(&*config).await?;
        self.ui_system.on_config_changed(&*config).await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Service for PersonalAssistant {
    fn name(&self) -> &'static str {
        "personal_assistant"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["ai_provider", "agent_framework", "context_engine", "memory", "tools"]
    }

    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }

    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the personal assistant with default configuration
pub async fn initialize_personal_assistant(
    ai_provider: Arc<AIProviderManager>,
    agent_framework: Arc<AgentFramework>,
) -> SymbioteResult<PersonalAssistant> {
    let config = AssistantConfig::default();
    PersonalAssistant::new(config, ai_provider, agent_framework).await
}

/// Initialize the personal assistant with custom configuration
pub async fn initialize_personal_assistant_with_config(
    config: AssistantConfig,
    ai_provider: Arc<AIProviderManager>,
    agent_framework: Arc<AgentFramework>,
) -> SymbioteResult<PersonalAssistant> {
    PersonalAssistant::new(config, ai_provider, agent_framework).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use symbiote_ai::AIProviderManager;
    use symbiote_agents::AgentFramework;

    #[tokio::test]
    async fn test_personal_assistant_creation() {
        let ai_provider = Arc::new(AIProviderManager::new().await.unwrap());
        let agent_framework = Arc::new(AgentFramework::new().await.unwrap());
        let config = AssistantConfig::default();
        let assistant = PersonalAssistant::new(config, ai_provider, agent_framework).await;
        assert!(assistant.is_ok());
    }

    #[tokio::test]
    async fn test_personal_assistant_lifecycle() {
        let ai_provider = Arc::new(AIProviderManager::new().await.unwrap());
        let agent_framework = Arc::new(AgentFramework::new().await.unwrap());
        let config = AssistantConfig::default();
        let assistant = PersonalAssistant::new(config, ai_provider, agent_framework).await.unwrap();

        let start_result = assistant.start().await;
        assert!(start_result.is_ok());

        let stop_result = assistant.stop().await;
        assert!(stop_result.is_ok());
    }
}

impl PersonalAssistant {
    /// Create a new Personal Assistant instance
    pub async fn new(config: AssistantConfig) -> SymbioteResult<Self> {
        let orchestrator = MasterOrchestrator::new(config.orchestrator.clone()).await?;
        let chat_interface = ChatInterface::new(config.chat.clone())?;
        let context_manager = ContextManager::new(config.context.clone())?;
        let personalization_engine = PersonalizationEngine::new(config.personalization.clone())?;
        let multimodal_processor = MultiModalProcessor::new(config.multimodal.clone())?;
        let agent_coordinator = AgentCoordinator::new(config.agents.clone()).await?;
        let tool_manager = ToolManager::new(config.tools.clone())?;

        Ok(Self {
            orchestrator,
            chat_interface,
            context_manager,
            personalization_engine,
            multimodal_processor,
            agent_coordinator,
            tool_manager,
            config,
        })
    }

    /// Process a user message and return an assistant response
    pub async fn process_message(&mut self, message: UserMessage) -> SymbioteResult<AssistantResponse> {
        // Get current context
        let context = self.context_manager.get_current_context().await?;
        
        // Route the request through the orchestrator
        let orchestrated_response = self.orchestrator
            .process_request(message.into(), &context)
            .await?;
        
        // Process through chat interface
        self.chat_interface.process_orchestrated_response(orchestrated_response).await
    }

    /// Process voice input
    pub async fn process_voice(&mut self, audio: AudioInput) -> SymbioteResult<AssistantResponse> {
        let processed_voice = self.multimodal_processor.process_voice(audio, &self.get_context().await?).await?;
        let message = UserMessage::from_voice(processed_voice);
        self.process_message(message).await
    }

    /// Process image input
    pub async fn process_image(&mut self, image: ImageInput) -> SymbioteResult<AssistantResponse> {
        let processed_image = self.multimodal_processor.process_image(image, &self.get_context().await?).await?;
        let message = UserMessage::from_image(processed_image);
        self.process_message(message).await
    }

    /// Process file input
    pub async fn process_file(&mut self, file: FileInput) -> SymbioteResult<AssistantResponse> {
        let processed_file = self.multimodal_processor.process_file(file, &self.get_context().await?).await?;
        let message = UserMessage::from_file(processed_file);
        self.process_message(message).await
    }

    /// Get personalized suggestions for the current context
    pub async fn get_suggestions(&self, context: UserContext) -> SymbioteResult<Vec<Suggestion>> {
        self.personalization_engine.get_personalized_suggestions(&context).await
    }

    /// Update the current context
    pub async fn update_context(&mut self, context_update: context::ContextUpdate) -> SymbioteResult<()> {
        self.context_manager.update_context(context_update).await
    }

    /// Set the current workspace
    pub async fn set_workspace(&mut self, workspace: WorkspaceInfo) -> SymbioteResult<()> {
        self.context_manager.update_workspace(workspace).await
    }

    /// Get conversation history
    pub async fn get_conversation_history(&self, filter: chat::HistoryFilter) -> SymbioteResult<Vec<chat::ConversationEntry>> {
        self.chat_interface.get_conversation_history(filter).await
    }

    /// Export user preferences
    pub async fn export_preferences(&self) -> SymbioteResult<UserPreferences> {
        self.personalization_engine.export_user_model().await
    }

    /// Import user preferences
    pub async fn import_preferences(&mut self, preferences: UserPreferences) -> SymbioteResult<()> {
        self.personalization_engine.import_user_model(preferences).await
    }

    // Private helper methods
    async fn get_context(&self) -> SymbioteResult<UserContext> {
        self.context_manager.get_current_context().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_assistant_creation() {
        let config = AssistantConfig::default();
        let assistant = PersonalAssistant::new(config).await;
        assert!(assistant.is_ok());
    }
}
