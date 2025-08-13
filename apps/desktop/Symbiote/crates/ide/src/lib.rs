//! # Symbiote IDE
//!
//! AI-native integrated development environment that revolutionizes software development with:
//! - AI-first development assistance
//! - Context-aware code completion and generation
//! - Multi-language support with LSP integration
//! - Integrated debugging and testing
//! - Real-time collaboration
//! - Performance optimization and analysis

#![deny(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod editor;
pub mod ai_assistance;
pub mod project_management;
pub mod debugging;
pub mod testing;
pub mod collaboration;
pub mod extensions;
pub mod ui;
pub mod server;
pub mod config;

// Re-export main types
pub use editor::{TextEditor, EditorEngine, EditorConfig};
pub use ai_assistance::{AiAssistant, CodeCompletion, CodeGeneration};
pub use project_management::{ProjectManager, Workspace, Project};
pub use debugging::{Debugger, BreakpointManager, VariableInspector};
pub use testing::{TestRunner, TestFramework, TestResult};
pub use collaboration::{CollaborationEngine, LiveShare, CodeReview};
pub use extensions::{ExtensionManager, Plugin, PluginApi};
pub use ui::{UILayoutSystem, Panel, Theme};
pub use server::{IdeServer, WebSocketHandler, ApiRouter};
pub use config::{IdeConfig, EditorSettings, AiSettings};

// Core dependencies
use symbiote_core::{SymbioteResult, SymbioteError, Service};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main IDE engine that orchestrates all IDE functionality
pub struct IdeEngine {
    /// Core editor functionality
    pub editor: Arc<EditorEngine>,
    
    /// AI-powered assistance
    pub ai_assistant: Arc<AiAssistant>,
    
    /// Project and workspace management
    pub project_manager: Arc<ProjectManager>,
    
    /// Debugging capabilities
    pub debugger: Arc<Debugger>,
    
    /// Testing framework
    pub test_runner: Arc<TestRunner>,
    
    /// Real-time collaboration
    pub collaboration: Arc<CollaborationEngine>,
    
    /// Extension system
    pub extensions: Arc<ExtensionManager>,
    
    /// UI layout system
    pub ui_system: Arc<UILayoutSystem>,
    
    /// Configuration
    config: Arc<RwLock<IdeConfig>>,
}

impl IdeEngine {
    /// Create a new IDE engine instance
    pub async fn new(config: IdeConfig) -> SymbioteResult<Self> {
        let config = Arc::new(RwLock::new(config));
        
        // Initialize core components
        let editor = Arc::new(EditorEngine::new(config.clone()).await?);
        let ai_assistant = Arc::new(AiAssistant::new(config.clone()).await?);
        let project_manager = Arc::new(ProjectManager::new(config.clone()).await?);
        let debugger = Arc::new(Debugger::new(config.clone()).await?);
        let test_runner = Arc::new(TestRunner::new(config.clone()).await?);
        let collaboration = Arc::new(CollaborationEngine::new(config.clone()).await?);
        let extensions = Arc::new(ExtensionManager::new(config.clone()).await?);
        let ui_system = Arc::new(UILayoutSystem::new(config.clone()).await?);
        
        Ok(Self {
            editor,
            ai_assistant,
            project_manager,
            debugger,
            test_runner,
            collaboration,
            extensions,
            ui_system,
            config,
        })
    }
    
    /// Start the IDE engine
    pub async fn start(&self) -> SymbioteResult<()> {
        tracing::info!("Starting Symbiote IDE engine");
        
        // Initialize all components
        self.editor.initialize().await?;
        self.ai_assistant.initialize().await?;
        self.project_manager.initialize().await?;
        self.debugger.initialize().await?;
        self.test_runner.initialize().await?;
        self.collaboration.initialize().await?;
        self.extensions.initialize().await?;
        self.ui_system.initialize().await?;
        
        tracing::info!("Symbiote IDE engine started successfully");
        Ok(())
    }
    
    /// Stop the IDE engine
    pub async fn stop(&self) -> SymbioteResult<()> {
        tracing::info!("Stopping Symbiote IDE engine");
        
        // Shutdown all components in reverse order
        self.ui_system.shutdown().await?;
        self.extensions.shutdown().await?;
        self.collaboration.shutdown().await?;
        self.test_runner.shutdown().await?;
        self.debugger.shutdown().await?;
        self.project_manager.shutdown().await?;
        self.ai_assistant.shutdown().await?;
        self.editor.shutdown().await?;
        
        tracing::info!("Symbiote IDE engine stopped successfully");
        Ok(())
    }
    
    /// Get the current configuration
    pub async fn get_config(&self) -> IdeConfig {
        self.config.read().await.clone()
    }
    
    /// Update the configuration
    pub async fn update_config(&self, new_config: IdeConfig) -> SymbioteResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        
        // Notify all components of config change
        self.editor.on_config_changed(&*config).await?;
        self.ai_assistant.on_config_changed(&*config).await?;
        self.project_manager.on_config_changed(&*config).await?;
        self.debugger.on_config_changed(&*config).await?;
        self.test_runner.on_config_changed(&*config).await?;
        self.collaboration.on_config_changed(&*config).await?;
        self.extensions.on_config_changed(&*config).await?;
        self.ui_system.on_config_changed(&*config).await?;
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl Service for IdeEngine {
    fn name(&self) -> &'static str {
        "ide_engine"
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["ai_provider", "context_engine", "storage"]
    }
    
    async fn initialize(&mut self) -> SymbioteResult<()> {
        self.start().await
    }
    
    async fn shutdown(&mut self) -> SymbioteResult<()> {
        self.stop().await
    }
}

/// Initialize the IDE with default configuration
pub async fn initialize_ide() -> SymbioteResult<IdeEngine> {
    let config = IdeConfig::default();
    IdeEngine::new(config).await
}

/// Initialize the IDE with custom configuration
pub async fn initialize_ide_with_config(config: IdeConfig) -> SymbioteResult<IdeEngine> {
    IdeEngine::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ide_engine_creation() {
        let config = IdeConfig::default();
        let ide = IdeEngine::new(config).await;
        assert!(ide.is_ok());
    }

    #[tokio::test]
    async fn test_ide_engine_lifecycle() {
        let config = IdeConfig::default();
        let ide = IdeEngine::new(config).await.unwrap();
        
        let start_result = ide.start().await;
        assert!(start_result.is_ok());
        
        let stop_result = ide.stop().await;
        assert!(stop_result.is_ok());
    }
}
