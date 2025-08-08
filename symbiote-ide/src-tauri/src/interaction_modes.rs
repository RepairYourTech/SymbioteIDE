use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use anyhow::Result;
use uuid::Uuid;

/// Core interaction modes that define how users interact with SymbioteIDE
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InteractionMode {
    /// AI takes full control - minimal user intervention
    Easy,
    /// Collaborative development - AI suggests, user approves
    Interactive,
    /// Traditional IDE with AI assistance - user drives
    Manual,
}

impl Default for InteractionMode {
    fn default() -> Self {
        InteractionMode::Interactive
    }
}

/// Configuration for each interaction mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub autonomy_level: AutonomyLevel,
    pub ui_complexity: UIComplexity,
    pub agent_visibility: AgentVisibility,
    pub decision_making: DecisionMaking,
    pub context_strategy: ContextStrategy,
    pub automation_level: AutomationLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// Easy mode - AI decides everything
    Full,
    /// Interactive mode - AI suggests, user approves
    Collaborative,
    /// Manual mode - User drives, AI assists
    Assistive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIComplexity {
    /// Minimal UI for Easy mode
    Minimal,
    /// Collaborative UI with approval workflows
    Collaborative,
    /// Full IDE interface for Manual mode
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentVisibility {
    /// Agents work behind the scenes
    Hidden,
    /// Show agent suggestions and recommendations
    Suggestions,
    /// Full agent activity and status display
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionMaking {
    /// AI makes all decisions automatically
    Automatic,
    /// AI suggests, user approves major decisions
    Guided,
    /// User makes all decisions, AI provides assistance
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextStrategy {
    /// Aggressive context collection for full AI control
    Comprehensive,
    /// Balanced context for collaborative work
    Balanced,
    /// Minimal context, user provides direction
    Minimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutomationLevel {
    /// Full automation - AI handles everything
    Full,
    /// Guided automation with user checkpoints
    Guided,
    /// Manual control with AI assistance
    Manual,
}

/// Events broadcast when modes change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeChangeEvent {
    pub session_id: Uuid,
    pub old_mode: InteractionMode,
    pub new_mode: InteractionMode,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub config: ModeConfig,
}

/// Agent behavior adapter for mode-specific behaviors
pub struct AgentBehaviorAdapter {
    current_behaviors: HashMap<String, AgentBehaviorConfig>,
    mode_behavior_templates: HashMap<InteractionMode, HashMap<String, AgentBehaviorConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBehaviorConfig {
    pub autonomy_level: AutonomyLevel,
    pub requires_approval: bool,
    pub max_actions_without_approval: u32,
    pub context_window_size: usize,
    pub proactive_suggestions: bool,
    pub auto_execute_safe_actions: bool,
}

impl AgentBehaviorAdapter {
    pub fn new() -> Self {
        let mut adapter = Self {
            current_behaviors: HashMap::new(),
            mode_behavior_templates: HashMap::new(),
        };
        
        adapter.initialize_mode_templates();
        adapter
    }

    fn initialize_mode_templates(&mut self) {
        // Easy Mode - AI takes full control
        let easy_config = AgentBehaviorConfig {
            autonomy_level: AutonomyLevel::Full,
            requires_approval: false,
            max_actions_without_approval: u32::MAX,
            context_window_size: 8192,
            proactive_suggestions: true,
            auto_execute_safe_actions: true,
        };

        // Interactive Mode - Collaborative development
        let interactive_config = AgentBehaviorConfig {
            autonomy_level: AutonomyLevel::Collaborative,
            requires_approval: true,
            max_actions_without_approval: 3,
            context_window_size: 4096,
            proactive_suggestions: true,
            auto_execute_safe_actions: false,
        };

        // Manual Mode - Traditional IDE with AI assistance
        let manual_config = AgentBehaviorConfig {
            autonomy_level: AutonomyLevel::Assistive,
            requires_approval: true,
            max_actions_without_approval: 1,
            context_window_size: 2048,
            proactive_suggestions: false,
            auto_execute_safe_actions: false,
        };

        // Apply to all agent types
        let agent_types = vec![
            "rust_specialist", "react_specialist", "debug_agent", "test_agent",
            "deployment_agent", "security_agent", "refactor_agent", "performance_optimizer",
            "researcher", "task_planner", "code_reviewer", "documentation_agent"
        ];

        for mode in [InteractionMode::Easy, InteractionMode::Interactive, InteractionMode::Manual] {
            let config = match mode {
                InteractionMode::Easy => easy_config.clone(),
                InteractionMode::Interactive => interactive_config.clone(),
                InteractionMode::Manual => manual_config.clone(),
            };

            let mut mode_behaviors = HashMap::new();
            for agent_type in &agent_types {
                mode_behaviors.insert(agent_type.to_string(), config.clone());
            }
            
            self.mode_behavior_templates.insert(mode, mode_behaviors);
        }
    }

    pub async fn adapt_agents(&mut self, mode: &InteractionMode) -> Result<()> {
        if let Some(mode_template) = self.mode_behavior_templates.get(mode) {
            self.current_behaviors = mode_template.clone();
            
            match mode {
                InteractionMode::Easy => {
                    self.enable_full_autonomy().await?;
                    self.minimize_user_prompts().await?;
                    self.enable_auto_decision_making().await?;
                },
                InteractionMode::Interactive => {
                    self.enable_collaborative_mode().await?;
                    self.enable_suggestion_mode().await?;
                    self.require_user_approval_for_major_changes().await?;
                },
                InteractionMode::Manual => {
                    self.enable_assistive_mode().await?;
                    self.enable_on_demand_assistance().await?;
                    self.user_drives_development().await?;
                }
            }
        }
        
        Ok(())
    }

    async fn enable_full_autonomy(&self) -> Result<()> {
        // Implementation for full autonomy mode
        println!("🤖 Enabling full AI autonomy - agents will work independently");
        Ok(())
    }

    async fn minimize_user_prompts(&self) -> Result<()> {
        // Implementation for minimal user interaction
        println!("🔇 Minimizing user prompts - AI will make decisions automatically");
        Ok(())
    }

    async fn enable_auto_decision_making(&self) -> Result<()> {
        // Implementation for automatic decision making
        println!("⚡ Enabling automatic decision making");
        Ok(())
    }

    async fn enable_collaborative_mode(&self) -> Result<()> {
        // Implementation for collaborative mode
        println!("🤝 Enabling collaborative mode - AI suggests, user approves");
        Ok(())
    }

    async fn enable_suggestion_mode(&self) -> Result<()> {
        // Implementation for suggestion mode
        println!("💡 Enabling suggestion mode - AI provides recommendations");
        Ok(())
    }

    async fn require_user_approval_for_major_changes(&self) -> Result<()> {
        // Implementation for approval requirements
        println!("✋ Requiring user approval for major changes");
        Ok(())
    }

    async fn enable_assistive_mode(&self) -> Result<()> {
        // Implementation for assistive mode
        println!("🛠️ Enabling assistive mode - AI provides help on demand");
        Ok(())
    }

    async fn enable_on_demand_assistance(&self) -> Result<()> {
        // Implementation for on-demand assistance
        println!("📞 Enabling on-demand AI assistance");
        Ok(())
    }

    async fn user_drives_development(&self) -> Result<()> {
        // Implementation for user-driven development
        println!("👨‍💻 User drives development - AI assists when requested");
        Ok(())
    }

    pub fn get_agent_behavior(&self, agent_type: &str) -> Option<&AgentBehaviorConfig> {
        self.current_behaviors.get(agent_type)
    }
}

/// Main mode manager that orchestrates interaction mode changes
pub struct ModeManager {
    current_mode: InteractionMode,
    mode_configs: HashMap<InteractionMode, ModeConfig>,
    agent_behavior_adapter: AgentBehaviorAdapter,
    event_sender: broadcast::Sender<ModeChangeEvent>,
    session_id: Uuid,
}

impl ModeManager {
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        let mut manager = Self {
            current_mode: InteractionMode::default(),
            mode_configs: HashMap::new(),
            agent_behavior_adapter: AgentBehaviorAdapter::new(),
            event_sender,
            session_id: Uuid::new_v4(),
        };
        
        manager.initialize_mode_configs();
        manager
    }

    fn initialize_mode_configs(&mut self) {
        // Easy Mode Configuration
        self.mode_configs.insert(
            InteractionMode::Easy,
            ModeConfig {
                autonomy_level: AutonomyLevel::Full,
                ui_complexity: UIComplexity::Minimal,
                agent_visibility: AgentVisibility::Hidden,
                decision_making: DecisionMaking::Automatic,
                context_strategy: ContextStrategy::Comprehensive,
                automation_level: AutomationLevel::Full,
            },
        );

        // Interactive Mode Configuration
        self.mode_configs.insert(
            InteractionMode::Interactive,
            ModeConfig {
                autonomy_level: AutonomyLevel::Collaborative,
                ui_complexity: UIComplexity::Collaborative,
                agent_visibility: AgentVisibility::Suggestions,
                decision_making: DecisionMaking::Guided,
                context_strategy: ContextStrategy::Balanced,
                automation_level: AutomationLevel::Guided,
            },
        );

        // Manual Mode Configuration
        self.mode_configs.insert(
            InteractionMode::Manual,
            ModeConfig {
                autonomy_level: AutonomyLevel::Assistive,
                ui_complexity: UIComplexity::Full,
                agent_visibility: AgentVisibility::Full,
                decision_making: DecisionMaking::Manual,
                context_strategy: ContextStrategy::Minimal,
                automation_level: AutomationLevel::Manual,
            },
        );
    }

    pub async fn switch_mode(&mut self, new_mode: InteractionMode) -> Result<()> {
        let old_mode = self.current_mode.clone();
        
        if old_mode == new_mode {
            return Ok(());
        }

        println!("🔄 Switching from {:?} to {:?} mode", old_mode, new_mode);

        // Adapt agent behaviors based on new mode
        self.agent_behavior_adapter.adapt_agents(&new_mode).await?;
        
        // Update context management strategy
        self.update_context_strategy(&new_mode).await?;
        
        // Update current mode
        self.current_mode = new_mode.clone();
        
        // Broadcast mode change event
        self.broadcast_mode_change(&old_mode, &new_mode).await?;
        
        println!("✅ Successfully switched to {:?} mode", new_mode);
        Ok(())
    }

    async fn update_context_strategy(&self, mode: &InteractionMode) -> Result<()> {
        match mode {
            InteractionMode::Easy => {
                println!("📊 Setting comprehensive context strategy for Easy mode");
                // Enable aggressive context collection
            },
            InteractionMode::Interactive => {
                println!("⚖️ Setting balanced context strategy for Interactive mode");
                // Enable balanced context collection
            },
            InteractionMode::Manual => {
                println!("🎯 Setting minimal context strategy for Manual mode");
                // Enable minimal context collection
            }
        }
        Ok(())
    }

    async fn broadcast_mode_change(&self, old_mode: &InteractionMode, new_mode: &InteractionMode) -> Result<()> {
        let config = self.mode_configs.get(new_mode).cloned().unwrap_or_else(|| {
            // Fallback config
            ModeConfig {
                autonomy_level: AutonomyLevel::Collaborative,
                ui_complexity: UIComplexity::Collaborative,
                agent_visibility: AgentVisibility::Suggestions,
                decision_making: DecisionMaking::Guided,
                context_strategy: ContextStrategy::Balanced,
                automation_level: AutomationLevel::Guided,
            }
        });

        let event = ModeChangeEvent {
            session_id: self.session_id,
            old_mode: old_mode.clone(),
            new_mode: new_mode.clone(),
            timestamp: chrono::Utc::now(),
            config,
        };

        // Broadcast to all subscribers
        let _ = self.event_sender.send(event);
        Ok(())
    }

    pub fn get_current_mode(&self) -> &InteractionMode {
        &self.current_mode
    }

    pub fn get_mode_config(&self, mode: &InteractionMode) -> Option<&ModeConfig> {
        self.mode_configs.get(mode)
    }

    pub fn get_current_config(&self) -> Option<&ModeConfig> {
        self.mode_configs.get(&self.current_mode)
    }

    pub fn subscribe_to_mode_changes(&self) -> broadcast::Receiver<ModeChangeEvent> {
        self.event_sender.subscribe()
    }

    pub fn get_agent_behavior(&self, agent_type: &str) -> Option<&AgentBehaviorConfig> {
        self.agent_behavior_adapter.get_agent_behavior(agent_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mode_switching() {
        let mut manager = ModeManager::new();
        
        // Test switching to Easy mode
        manager.switch_mode(InteractionMode::Easy).await.unwrap();
        assert_eq!(*manager.get_current_mode(), InteractionMode::Easy);
        
        // Test switching to Manual mode
        manager.switch_mode(InteractionMode::Manual).await.unwrap();
        assert_eq!(*manager.get_current_mode(), InteractionMode::Manual);
    }

    #[tokio::test]
    async fn test_agent_behavior_adaptation() {
        let mut adapter = AgentBehaviorAdapter::new();
        
        // Test Easy mode adaptation
        adapter.adapt_agents(&InteractionMode::Easy).await.unwrap();
        let config = adapter.get_agent_behavior("rust_specialist").unwrap();
        assert_eq!(config.autonomy_level, AutonomyLevel::Full);
        assert!(!config.requires_approval);
        
        // Test Manual mode adaptation
        adapter.adapt_agents(&InteractionMode::Manual).await.unwrap();
        let config = adapter.get_agent_behavior("rust_specialist").unwrap();
        assert_eq!(config.autonomy_level, AutonomyLevel::Assistive);
        assert!(config.requires_approval);
    }

    #[test]
    fn test_mode_configs() {
        let manager = ModeManager::new();
        
        let easy_config = manager.get_mode_config(&InteractionMode::Easy).unwrap();
        assert_eq!(easy_config.autonomy_level, AutonomyLevel::Full);
        assert_eq!(easy_config.ui_complexity, UIComplexity::Minimal);
        
        let manual_config = manager.get_mode_config(&InteractionMode::Manual).unwrap();
        assert_eq!(manual_config.autonomy_level, AutonomyLevel::Assistive);
        assert_eq!(manual_config.ui_complexity, UIComplexity::Full);
    }
}
