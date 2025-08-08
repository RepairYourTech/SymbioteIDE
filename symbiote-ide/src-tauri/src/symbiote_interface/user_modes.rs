// User Modes - Seamless IDE Mode Switching System
// Phase 2 Feature: Fast context switching between different development modes

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// User Modes - System for seamless switching between different IDE modes
pub struct UserModes {
    available_modes: Arc<RwLock<HashMap<String, ModeDefinition>>>,
    active_mode: Arc<RwLock<Option<String>>>,
    mode_history: Arc<RwLock<Vec<ModeTransition>>>,
    mode_contexts: Arc<RwLock<HashMap<String, ModeContext>>>,
    metrics: Arc<RwLock<UserModesMetrics>>,
}

/// Definition of an IDE mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeDefinition {
    pub mode_id: String,
    pub mode_name: String,
    pub mode_type: ModeType,
    pub description: String,
    pub ui_layout: UILayout,
    pub available_tools: Vec<String>,
    pub keyboard_shortcuts: HashMap<String, String>,
    pub performance_targets: ModePerformanceTargets,
}

/// Types of IDE modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModeType {
    Coding,
    Debugging,
    Testing,
    VisualBuilder,
    AIAssisted,
    Security,
    Performance,
    Custom(String),
}

/// UI layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UILayout {
    pub layout_id: String,
    pub panels: Vec<PanelConfiguration>,
    pub sidebars: Vec<SidebarConfiguration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfiguration {
    pub panel_id: String,
    pub panel_type: String,
    pub visible: bool,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarConfiguration {
    pub sidebar_id: String,
    pub position: SidebarPosition,
    pub width: f32,
    pub visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SidebarPosition {
    Left,
    Right,
}

/// Performance targets for mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModePerformanceTargets {
    pub activation_time: std::time::Duration,
    pub deactivation_time: std::time::Duration,
    pub memory_usage_limit: u64,
}

/// Mode transition tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeTransition {
    pub transition_id: String,
    pub from_mode: Option<String>,
    pub to_mode: String,
    pub trigger: TransitionTrigger,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration: Option<std::time::Duration>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionTrigger {
    UserAction,
    FileOpen,
    AIRecommendation,
    WorkflowStep,
}

/// Context preservation for modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeContext {
    pub mode_id: String,
    pub context_data: HashMap<String, serde_json::Value>,
    pub open_files: Vec<String>,
    pub cursor_positions: HashMap<String, CursorPosition>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

/// Performance metrics
#[derive(Debug, Default)]
pub struct UserModesMetrics {
    pub total_transitions: u32,
    pub successful_transitions: u32,
    pub failed_transitions: u32,
    pub average_transition_time: std::time::Duration,
    pub mode_usage_stats: HashMap<String, ModeUsageStats>,
}

#[derive(Debug, Default)]
pub struct ModeUsageStats {
    pub activation_count: u32,
    pub total_time_active: std::time::Duration,
    pub last_used: Option<DateTime<Utc>>,
}

impl UserModes {
    /// Create a new User Modes system
    pub fn new() -> Self {
        Self {
            available_modes: Arc::new(RwLock::new(HashMap::new())),
            active_mode: Arc::new(RwLock::new(None)),
            mode_history: Arc::new(RwLock::new(Vec::new())),
            mode_contexts: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(UserModesMetrics::default())),
        }
    }
    
    /// Initialize default IDE modes
    pub async fn initialize_default_modes(&self) -> Result<(), UserModesError> {
        let default_modes = vec![
            self.create_coding_mode(),
            self.create_debugging_mode(),
            self.create_visual_builder_mode(),
            self.create_testing_mode(),
            self.create_ai_assisted_mode(),
        ];
        
        let mut available_modes = self.available_modes.write().await;
        for mode in default_modes {
            available_modes.insert(mode.mode_id.clone(), mode);
        }
        
        Ok(())
    }
    
    /// Switch to a different mode with <1 second performance target
    pub async fn switch_mode(&self, mode_id: String, trigger: TransitionTrigger) -> Result<String, UserModesError> {
        let transition_id = Uuid::new_v4().to_string();
        let start_time = Utc::now();
        
        // Get current mode
        let current_mode = {
            let active_mode = self.active_mode.read().await;
            active_mode.clone()
        };
        
        // Validate mode exists
        {
            let available_modes = self.available_modes.read().await;
            if !available_modes.contains_key(&mode_id) {
                return Err(UserModesError::ModeNotFound);
            }
        }
        
        // Save current context
        if let Some(current) = &current_mode {
            self.save_mode_context(current).await?;
        }
        
        // Execute fast transition
        match self.execute_mode_transition(&mode_id).await {
            Ok(_) => {
                let end_time = Utc::now();
                let duration = end_time.signed_duration_since(start_time).to_std().unwrap_or_default();
                
                // Update active mode
                {
                    let mut active_mode = self.active_mode.write().await;
                    *active_mode = Some(mode_id.clone());
                }
                
                // Record successful transition
                let transition = ModeTransition {
                    transition_id: transition_id.clone(),
                    from_mode: current_mode,
                    to_mode: mode_id,
                    trigger,
                    started_at: start_time,
                    completed_at: Some(end_time),
                    duration: Some(duration),
                    success: true,
                };
                
                {
                    let mut history = self.mode_history.write().await;
                    history.push(transition);
                }
                
                // Update metrics
                self.update_transition_metrics(duration, true).await;
                
                Ok(transition_id)
            }
            Err(e) => {
                // Record failed transition
                let transition = ModeTransition {
                    transition_id: transition_id.clone(),
                    from_mode: current_mode,
                    to_mode: mode_id,
                    trigger,
                    started_at: start_time,
                    completed_at: Some(Utc::now()),
                    duration: None,
                    success: false,
                };
                
                {
                    let mut history = self.mode_history.write().await;
                    history.push(transition);
                }
                
                self.update_transition_metrics(std::time::Duration::from_secs(0), false).await;
                Err(e)
            }
        }
    }
    
    /// Execute mode transition with performance optimization
    async fn execute_mode_transition(&self, mode_id: &str) -> Result<(), UserModesError> {
        // Get mode definition
        let mode = {
            let available_modes = self.available_modes.read().await;
            available_modes.get(mode_id)
                .ok_or(UserModesError::ModeNotFound)?
                .clone()
        };
        
        // Fast UI layout switch
        self.apply_ui_layout(&mode.ui_layout).await?;
        
        // Restore mode context
        self.restore_mode_context(mode_id).await?;
        
        // Configure mode tools
        self.configure_mode_tools(&mode).await?;
        
        Ok(())
    }
    
    /// Save current mode context for fast restoration
    async fn save_mode_context(&self, mode_id: &str) -> Result<(), UserModesError> {
        let context = ModeContext {
            mode_id: mode_id.to_string(),
            context_data: HashMap::new(),
            open_files: Vec::new(), // Would capture actual open files
            cursor_positions: HashMap::new(), // Would capture actual cursor positions
            last_updated: Utc::now(),
        };
        
        let mut mode_contexts = self.mode_contexts.write().await;
        mode_contexts.insert(mode_id.to_string(), context);
        
        Ok(())
    }
    
    /// Restore mode context for fast switching
    async fn restore_mode_context(&self, mode_id: &str) -> Result<(), UserModesError> {
        let mode_contexts = self.mode_contexts.read().await;
        if let Some(_context) = mode_contexts.get(mode_id) {
            // Fast context restoration - would restore UI state, open files, etc.
        }
        Ok(())
    }
    
    /// Apply UI layout optimized for speed
    async fn apply_ui_layout(&self, _layout: &UILayout) -> Result<(), UserModesError> {
        // Fast UI layout application - would update panels, sidebars, etc.
        Ok(())
    }
    
    /// Configure mode tools quickly
    async fn configure_mode_tools(&self, _mode: &ModeDefinition) -> Result<(), UserModesError> {
        // Fast tool configuration - would update toolbars, shortcuts, etc.
        Ok(())
    }
    
    /// Get current active mode
    pub async fn get_active_mode(&self) -> Option<String> {
        let active_mode = self.active_mode.read().await;
        active_mode.clone()
    }
    
    /// Get available modes
    pub async fn get_available_modes(&self) -> Vec<ModeDefinition> {
        let available_modes = self.available_modes.read().await;
        available_modes.values().cloned().collect()
    }
    
    /// Get performance metrics
    pub async fn get_metrics(&self) -> UserModesMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    /// Update transition metrics
    async fn update_transition_metrics(&self, duration: std::time::Duration, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_transitions += 1;
        
        if success {
            metrics.successful_transitions += 1;
            
            // Update average transition time
            let total_time = metrics.average_transition_time * (metrics.successful_transitions - 1) + duration;
            metrics.average_transition_time = total_time / metrics.successful_transitions;
        } else {
            metrics.failed_transitions += 1;
        }
    }
    
    // Mode creation helpers
    fn create_coding_mode(&self) -> ModeDefinition {
        ModeDefinition {
            mode_id: "coding".to_string(),
            mode_name: "Coding".to_string(),
            mode_type: ModeType::Coding,
            description: "Standard code editing mode".to_string(),
            ui_layout: UILayout {
                layout_id: "coding_layout".to_string(),
                panels: vec![
                    PanelConfiguration {
                        panel_id: "editor".to_string(),
                        panel_type: "code_editor".to_string(),
                        visible: true,
                        width: 800.0,
                        height: 600.0,
                    }
                ],
                sidebars: vec![
                    SidebarConfiguration {
                        sidebar_id: "file_explorer".to_string(),
                        position: SidebarPosition::Left,
                        width: 250.0,
                        visible: true,
                    }
                ],
            },
            available_tools: vec!["syntax_highlighting".to_string(), "intellisense".to_string()],
            keyboard_shortcuts: HashMap::new(),
            performance_targets: ModePerformanceTargets {
                activation_time: std::time::Duration::from_millis(500),
                deactivation_time: std::time::Duration::from_millis(200),
                memory_usage_limit: 100 * 1024 * 1024, // 100MB
            },
        }
    }
    
    fn create_debugging_mode(&self) -> ModeDefinition {
        ModeDefinition {
            mode_id: "debugging".to_string(),
            mode_name: "Debugging".to_string(),
            mode_type: ModeType::Debugging,
            description: "Debug mode with breakpoints and inspection".to_string(),
            ui_layout: UILayout {
                layout_id: "debug_layout".to_string(),
                panels: vec![
                    PanelConfiguration {
                        panel_id: "debug_console".to_string(),
                        panel_type: "console".to_string(),
                        visible: true,
                        width: 800.0,
                        height: 200.0,
                    }
                ],
                sidebars: vec![
                    SidebarConfiguration {
                        sidebar_id: "variables".to_string(),
                        position: SidebarPosition::Right,
                        width: 300.0,
                        visible: true,
                    }
                ],
            },
            available_tools: vec!["breakpoints".to_string(), "variable_inspector".to_string()],
            keyboard_shortcuts: HashMap::new(),
            performance_targets: ModePerformanceTargets {
                activation_time: std::time::Duration::from_millis(800),
                deactivation_time: std::time::Duration::from_millis(300),
                memory_usage_limit: 200 * 1024 * 1024, // 200MB
            },
        }
    }
    
    fn create_visual_builder_mode(&self) -> ModeDefinition {
        ModeDefinition {
            mode_id: "visual_builder".to_string(),
            mode_name: "Visual Builder".to_string(),
            mode_type: ModeType::VisualBuilder,
            description: "Drag-and-drop visual development".to_string(),
            ui_layout: UILayout {
                layout_id: "visual_layout".to_string(),
                panels: vec![
                    PanelConfiguration {
                        panel_id: "canvas".to_string(),
                        panel_type: "visual_canvas".to_string(),
                        visible: true,
                        width: 1000.0,
                        height: 700.0,
                    }
                ],
                sidebars: vec![
                    SidebarConfiguration {
                        sidebar_id: "component_palette".to_string(),
                        position: SidebarPosition::Left,
                        width: 200.0,
                        visible: true,
                    },
                    SidebarConfiguration {
                        sidebar_id: "properties".to_string(),
                        position: SidebarPosition::Right,
                        width: 250.0,
                        visible: true,
                    }
                ],
            },
            available_tools: vec!["component_palette".to_string(), "property_editor".to_string()],
            keyboard_shortcuts: HashMap::new(),
            performance_targets: ModePerformanceTargets {
                activation_time: std::time::Duration::from_millis(1000),
                deactivation_time: std::time::Duration::from_millis(400),
                memory_usage_limit: 300 * 1024 * 1024, // 300MB
            },
        }
    }
    
    fn create_testing_mode(&self) -> ModeDefinition {
        ModeDefinition {
            mode_id: "testing".to_string(),
            mode_name: "Testing".to_string(),
            mode_type: ModeType::Testing,
            description: "Test runner and coverage analysis".to_string(),
            ui_layout: UILayout {
                layout_id: "testing_layout".to_string(),
                panels: vec![
                    PanelConfiguration {
                        panel_id: "test_results".to_string(),
                        panel_type: "test_runner".to_string(),
                        visible: true,
                        width: 800.0,
                        height: 400.0,
                    }
                ],
                sidebars: vec![
                    SidebarConfiguration {
                        sidebar_id: "test_explorer".to_string(),
                        position: SidebarPosition::Left,
                        width: 250.0,
                        visible: true,
                    }
                ],
            },
            available_tools: vec!["test_runner".to_string(), "coverage_analyzer".to_string()],
            keyboard_shortcuts: HashMap::new(),
            performance_targets: ModePerformanceTargets {
                activation_time: std::time::Duration::from_millis(600),
                deactivation_time: std::time::Duration::from_millis(250),
                memory_usage_limit: 150 * 1024 * 1024, // 150MB
            },
        }
    }
    
    fn create_ai_assisted_mode(&self) -> ModeDefinition {
        ModeDefinition {
            mode_id: "ai_assisted".to_string(),
            mode_name: "AI Assisted".to_string(),
            mode_type: ModeType::AIAssisted,
            description: "AI-powered development assistance".to_string(),
            ui_layout: UILayout {
                layout_id: "ai_layout".to_string(),
                panels: vec![
                    PanelConfiguration {
                        panel_id: "ai_chat".to_string(),
                        panel_type: "ai_assistant".to_string(),
                        visible: true,
                        width: 400.0,
                        height: 600.0,
                    }
                ],
                sidebars: vec![
                    SidebarConfiguration {
                        sidebar_id: "ai_suggestions".to_string(),
                        position: SidebarPosition::Right,
                        width: 300.0,
                        visible: true,
                    }
                ],
            },
            available_tools: vec!["ai_chat".to_string(), "code_suggestions".to_string(), "auto_completion".to_string()],
            keyboard_shortcuts: HashMap::new(),
            performance_targets: ModePerformanceTargets {
                activation_time: std::time::Duration::from_millis(700),
                deactivation_time: std::time::Duration::from_millis(300),
                memory_usage_limit: 250 * 1024 * 1024, // 250MB
            },
        }
    }
}

/// User Modes error types
#[derive(Debug, thiserror::Error)]
pub enum UserModesError {
    #[error("Mode not found")]
    ModeNotFound,
    #[error("Transition failed")]
    TransitionFailed,
    #[error("Context save failed")]
    ContextSaveFailed,
    #[error("Context restore failed")]
    ContextRestoreFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_user_modes_creation() {
        let user_modes = UserModes::new();
        user_modes.initialize_default_modes().await.unwrap();
        
        let modes = user_modes.get_available_modes().await;
        assert_eq!(modes.len(), 5);
    }
    
    #[tokio::test]
    async fn test_mode_switching() {
        let user_modes = UserModes::new();
        user_modes.initialize_default_modes().await.unwrap();
        
        let transition_id = user_modes.switch_mode(
            "coding".to_string(),
            TransitionTrigger::UserAction,
        ).await.unwrap();
        
        assert!(!transition_id.is_empty());
        
        let active_mode = user_modes.get_active_mode().await;
        assert_eq!(active_mode, Some("coding".to_string()));
        
        // Test fast switching to debugging mode
        let debug_transition = user_modes.switch_mode(
            "debugging".to_string(),
            TransitionTrigger::UserAction,
        ).await.unwrap();
        
        assert!(!debug_transition.is_empty());
        
        let metrics = user_modes.get_metrics().await;
        assert_eq!(metrics.total_transitions, 2);
        assert_eq!(metrics.successful_transitions, 2);
        
        // Verify transition time is under 1 second
        assert!(metrics.average_transition_time < std::time::Duration::from_secs(1));
    }
}
