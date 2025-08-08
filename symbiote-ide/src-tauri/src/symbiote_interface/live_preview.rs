// Advanced Live Preview System - Real-time Code/Component Preview & Live Editing
// Phase 4 Feature: Revolutionary live preview with hot reload and element inspection

use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Advanced Live Preview System
pub struct LivePreviewSystem {
    // Preview management
    preview_manager: PreviewManager,
    preview_sessions: Arc<RwLock<HashMap<String, PreviewSession>>>,
    
    // Hot reload system
    hot_reload_engine: HotReloadEngine,
    file_watcher: FileWatcher,
    
    // Element inspection
    element_inspector: ElementInspector,
    dom_analyzer: DOMAnalyzer,
    
    // Live editing
    live_editor: LiveEditor,
    property_editor: PropertyEditor,
    
    // Synchronization
    sync_engine: SyncEngine,
    conflict_resolver: ConflictResolver,
    
    // Performance optimization
    render_optimizer: RenderOptimizer,
    cache_manager: CacheManager,
    
    // Performance metrics
    metrics: Arc<RwLock<LivePreviewMetrics>>,
}

/// Preview session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewSession {
    pub session_id: String,
    pub session_name: String,
    pub project_path: PathBuf,
    pub entry_point: PathBuf,
    pub framework: PreviewFramework,
    pub configuration: PreviewConfiguration,
    pub status: PreviewStatus,
    pub preview_url: String,
    pub websocket_url: String,
    pub watched_files: Vec<PathBuf>,
    pub active_inspections: Vec<ElementInspection>,
    pub live_edits: Vec<LiveEdit>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub performance_stats: PreviewPerformanceStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreviewFramework {
    React,
    Vue,
    Angular,
    Svelte,
    Vanilla,
    NextJS,
    Nuxt,
    SvelteKit,
    Astro,
    Static,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfiguration {
    pub port: u16,
    pub host: String,
    pub auto_reload: bool,
    pub hot_module_replacement: bool,
    pub source_maps: bool,
    pub css_injection: bool,
    pub js_injection: bool,
    pub error_overlay: bool,
    pub performance_monitoring: bool,
    pub accessibility_checks: bool,
    pub responsive_testing: bool,
    pub browser_sync: bool,
    pub custom_headers: HashMap<String, String>,
    pub proxy_rules: Vec<ProxyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRule {
    pub path: String,
    pub target: String,
    pub change_origin: bool,
    pub rewrite: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreviewStatus {
    Starting,
    Running,
    Reloading,
    Error,
    Stopped,
}

/// Element inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInspection {
    pub inspection_id: String,
    pub element_selector: String,
    pub element_path: Vec<String>,
    pub element_info: ElementInfo,
    pub computed_styles: HashMap<String, String>,
    pub box_model: BoxModel,
    pub accessibility_info: AccessibilityInfo,
    pub performance_metrics: ElementPerformanceMetrics,
    pub source_location: Option<SourceLocation>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementInfo {
    pub tag_name: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub inner_html: String,
    pub outer_html: String,
    pub children_count: u32,
    pub parent_selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxModel {
    pub content: Rectangle,
    pub padding: Rectangle,
    pub border: Rectangle,
    pub margin: Rectangle,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rectangle {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityInfo {
    pub role: Option<String>,
    pub aria_label: Option<String>,
    pub aria_describedby: Option<String>,
    pub tabindex: Option<i32>,
    pub focusable: bool,
    pub keyboard_accessible: bool,
    pub screen_reader_accessible: bool,
    pub color_contrast_ratio: Option<f64>,
    pub accessibility_violations: Vec<AccessibilityViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityViolation {
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub description: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationType {
    ColorContrast,
    MissingAltText,
    MissingLabel,
    KeyboardNavigation,
    FocusManagement,
    SemanticStructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementPerformanceMetrics {
    pub render_time: std::time::Duration,
    pub layout_time: std::time::Duration,
    pub paint_time: std::time::Duration,
    pub memory_usage: u64,
    pub event_listeners: u32,
    pub dom_updates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file_path: PathBuf,
    pub line_number: u32,
    pub column_number: u32,
    pub component_name: Option<String>,
}

/// Live editing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveEdit {
    pub edit_id: String,
    pub edit_type: LiveEditType,
    pub target_selector: String,
    pub property: String,
    pub old_value: String,
    pub new_value: String,
    pub source_file: Option<PathBuf>,
    pub applied_at: DateTime<Utc>,
    pub reverted: bool,
    pub sync_status: SyncStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiveEditType {
    StyleProperty,
    Attribute,
    TextContent,
    InnerHTML,
    ClassList,
    ComponentProp,
    StateValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    Applied,
    Failed,
    Conflicted,
    Reverted,
}

/// Hot reload system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadEvent {
    pub event_id: String,
    pub event_type: HotReloadEventType,
    pub affected_files: Vec<PathBuf>,
    pub change_type: ChangeType,
    pub timestamp: DateTime<Utc>,
    pub reload_strategy: ReloadStrategy,
    pub dependencies: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HotReloadEventType {
    FileChanged,
    FileAdded,
    FileDeleted,
    FileRenamed,
    DependencyChanged,
    ConfigChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Content,
    Style,
    Script,
    Asset,
    Config,
    Dependency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReloadStrategy {
    HotModuleReplacement,
    LiveReload,
    FullReload,
    IncrementalUpdate,
    ComponentRefresh,
}

/// Responsive testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsiveTestSession {
    pub session_id: String,
    pub viewports: Vec<Viewport>,
    pub current_viewport: String,
    pub orientation: Orientation,
    pub device_emulation: Option<DeviceEmulation>,
    pub touch_simulation: bool,
    pub network_throttling: Option<NetworkThrottling>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub device_pixel_ratio: f64,
    pub is_mobile: bool,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEmulation {
    pub device_name: String,
    pub user_agent: String,
    pub viewport: Viewport,
    pub touch_enabled: bool,
    pub mobile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkThrottling {
    pub download_speed: u64, // bytes per second
    pub upload_speed: u64,
    pub latency: std::time::Duration,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Offline,
    Slow3G,
    Fast3G,
    Slow4G,
    Fast4G,
    WiFi,
    Custom,
}

/// Performance monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPerformanceStats {
    pub page_load_time: std::time::Duration,
    pub first_contentful_paint: std::time::Duration,
    pub largest_contentful_paint: std::time::Duration,
    pub cumulative_layout_shift: f64,
    pub first_input_delay: std::time::Duration,
    pub memory_usage: MemoryUsage,
    pub network_requests: Vec<NetworkRequest>,
    pub javascript_errors: Vec<JavaScriptError>,
    pub console_logs: Vec<ConsoleLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub used_heap_size: u64,
    pub total_heap_size: u64,
    pub heap_size_limit: u64,
    pub external_memory: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub status_code: u16,
    pub response_time: std::time::Duration,
    pub response_size: u64,
    pub cached: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaScriptError {
    pub message: String,
    pub stack_trace: String,
    pub file_name: String,
    pub line_number: u32,
    pub column_number: u32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLog {
    pub level: LogLevel,
    pub message: String,
    pub args: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Log,
    Info,
    Warn,
    Error,
    Debug,
}

/// Supporting systems
pub struct PreviewManager;
pub struct HotReloadEngine;
pub struct FileWatcher;
pub struct ElementInspector;
pub struct DOMAnalyzer;
pub struct LiveEditor;
pub struct PropertyEditor;
pub struct SyncEngine;
pub struct ConflictResolver;
pub struct RenderOptimizer;
pub struct CacheManager;

/// Performance metrics
#[derive(Debug, Default)]
pub struct LivePreviewMetrics {
    pub total_sessions: u32,
    pub active_sessions: u32,
    pub total_hot_reloads: u32,
    pub total_live_edits: u32,
    pub average_reload_time: std::time::Duration,
    pub average_sync_time: std::time::Duration,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
    pub user_satisfaction: f64,
}

impl LivePreviewSystem {
    /// Create a new Live Preview System
    pub fn new() -> Self {
        Self {
            preview_manager: PreviewManager,
            preview_sessions: Arc::new(RwLock::new(HashMap::new())),
            hot_reload_engine: HotReloadEngine,
            file_watcher: FileWatcher,
            element_inspector: ElementInspector,
            dom_analyzer: DOMAnalyzer,
            live_editor: LiveEditor,
            property_editor: PropertyEditor,
            sync_engine: SyncEngine,
            conflict_resolver: ConflictResolver,
            render_optimizer: RenderOptimizer,
            cache_manager: CacheManager,
            metrics: Arc::new(RwLock::new(LivePreviewMetrics::default())),
        }
    }
    
    /// Start a new preview session
    pub async fn start_preview_session(
        &self,
        session_name: String,
        project_path: PathBuf,
        entry_point: PathBuf,
        framework: PreviewFramework,
        configuration: PreviewConfiguration,
    ) -> Result<String, LivePreviewError> {
        let session_id = Uuid::new_v4().to_string();
        
        // Generate preview URL
        let preview_url = format!("http://{}:{}", configuration.host, configuration.port);
        let websocket_url = format!("ws://{}:{}/ws", configuration.host, configuration.port + 1);
        
        let session = PreviewSession {
            session_id: session_id.clone(),
            session_name,
            project_path: project_path.clone(),
            entry_point,
            framework,
            configuration,
            status: PreviewStatus::Starting,
            preview_url,
            websocket_url,
            watched_files: Vec::new(),
            active_inspections: Vec::new(),
            live_edits: Vec::new(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            performance_stats: PreviewPerformanceStats::default(),
        };
        
        // Store session
        {
            let mut sessions = self.preview_sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }
        
        // Start preview server
        self.start_preview_server(&session_id).await?;
        
        // Setup file watching
        self.setup_file_watching(&session_id, &project_path).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_sessions += 1;
            metrics.active_sessions += 1;
        }
        
        Ok(session_id)
    }
    
    /// Start preview server
    async fn start_preview_server(&self, session_id: &str) -> Result<(), LivePreviewError> {
        // Mock server startup
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Update session status
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.status = PreviewStatus::Running;
                session.last_activity = Utc::now();
            }
        }
        
        Ok(())
    }
    
    /// Setup file watching for hot reload
    async fn setup_file_watching(&self, session_id: &str, project_path: &PathBuf) -> Result<(), LivePreviewError> {
        // Mock file watching setup
        let watched_files = vec![
            project_path.join("src"),
            project_path.join("public"),
            project_path.join("styles"),
        ];
        
        // Update session with watched files
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.watched_files = watched_files;
            }
        }
        
        Ok(())
    }
    
    /// Inspect element in preview
    pub async fn inspect_element(
        &self,
        session_id: String,
        element_selector: String,
    ) -> Result<String, LivePreviewError> {
        let inspection_id = Uuid::new_v4().to_string();
        
        // Mock element inspection
        let inspection = ElementInspection {
            inspection_id: inspection_id.clone(),
            element_selector: element_selector.clone(),
            element_path: vec!["html".to_string(), "body".to_string(), "div".to_string()],
            element_info: ElementInfo {
                tag_name: "div".to_string(),
                id: Some("main-content".to_string()),
                classes: vec!["container".to_string(), "active".to_string()],
                attributes: HashMap::from([
                    ("data-testid".to_string(), "main-container".to_string()),
                    ("role".to_string(), "main".to_string()),
                ]),
                text_content: Some("Main content area".to_string()),
                inner_html: "<p>Content here</p>".to_string(),
                outer_html: "<div id=\"main-content\" class=\"container active\"><p>Content here</p></div>".to_string(),
                children_count: 1,
                parent_selector: Some("body".to_string()),
            },
            computed_styles: HashMap::from([
                ("display".to_string(), "block".to_string()),
                ("width".to_string(), "100%".to_string()),
                ("padding".to_string(), "20px".to_string()),
                ("background-color".to_string(), "rgb(255, 255, 255)".to_string()),
            ]),
            box_model: BoxModel {
                content: Rectangle { top: 0.0, right: 800.0, bottom: 600.0, left: 0.0, width: 800.0, height: 600.0 },
                padding: Rectangle { top: -20.0, right: 820.0, bottom: 620.0, left: -20.0, width: 840.0, height: 640.0 },
                border: Rectangle { top: -21.0, right: 821.0, bottom: 621.0, left: -21.0, width: 842.0, height: 642.0 },
                margin: Rectangle { top: -21.0, right: 821.0, bottom: 621.0, left: -21.0, width: 842.0, height: 642.0 },
                position: Position { x: 0.0, y: 0.0, z_index: 0 },
            },
            accessibility_info: AccessibilityInfo {
                role: Some("main".to_string()),
                aria_label: None,
                aria_describedby: None,
                tabindex: None,
                focusable: false,
                keyboard_accessible: true,
                screen_reader_accessible: true,
                color_contrast_ratio: Some(4.5),
                accessibility_violations: Vec::new(),
            },
            performance_metrics: ElementPerformanceMetrics {
                render_time: std::time::Duration::from_millis(2),
                layout_time: std::time::Duration::from_millis(1),
                paint_time: std::time::Duration::from_millis(1),
                memory_usage: 1024,
                event_listeners: 0,
                dom_updates: 1,
            },
            source_location: Some(SourceLocation {
                file_path: PathBuf::from("src/components/MainContent.tsx"),
                line_number: 15,
                column_number: 8,
                component_name: Some("MainContent".to_string()),
            }),
            created_at: Utc::now(),
        };
        
        // Store inspection
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.active_inspections.push(inspection);
                session.last_activity = Utc::now();
            }
        }
        
        Ok(inspection_id)
    }
    
    /// Apply live edit to element
    pub async fn apply_live_edit(
        &self,
        session_id: String,
        target_selector: String,
        edit_type: LiveEditType,
        property: String,
        new_value: String,
    ) -> Result<String, LivePreviewError> {
        let edit_id = Uuid::new_v4().to_string();
        
        // Mock getting old value
        let old_value = "previous-value".to_string();
        
        let live_edit = LiveEdit {
            edit_id: edit_id.clone(),
            edit_type,
            target_selector,
            property,
            old_value,
            new_value,
            source_file: Some(PathBuf::from("src/components/Component.tsx")),
            applied_at: Utc::now(),
            reverted: false,
            sync_status: SyncStatus::Applied,
        };
        
        // Store live edit
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.live_edits.push(live_edit);
                session.last_activity = Utc::now();
            }
        }
        
        // Sync to source files
        self.sync_edit_to_source(&edit_id).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_live_edits += 1;
        }
        
        Ok(edit_id)
    }
    
    /// Sync live edit to source files
    async fn sync_edit_to_source(&self, _edit_id: &str) -> Result<(), LivePreviewError> {
        // Mock synchronization
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(())
    }
    
    /// Trigger hot reload
    pub async fn trigger_hot_reload(
        &self,
        session_id: String,
        changed_files: Vec<PathBuf>,
        change_type: ChangeType,
    ) -> Result<(), LivePreviewError> {
        let event = HotReloadEvent {
            event_id: Uuid::new_v4().to_string(),
            event_type: HotReloadEventType::FileChanged,
            affected_files: changed_files,
            change_type,
            timestamp: Utc::now(),
            reload_strategy: ReloadStrategy::HotModuleReplacement,
            dependencies: Vec::new(),
        };
        
        // Process hot reload
        self.process_hot_reload_event(&session_id, event).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_hot_reloads += 1;
        }
        
        Ok(())
    }
    
    /// Process hot reload event
    async fn process_hot_reload_event(&self, session_id: &str, _event: HotReloadEvent) -> Result<(), LivePreviewError> {
        // Update session status
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.status = PreviewStatus::Reloading;
            }
        }
        
        // Mock reload processing
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        // Update session status back to running
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.status = PreviewStatus::Running;
                session.last_activity = Utc::now();
            }
        }
        
        Ok(())
    }
    
    /// Get preview session
    pub async fn get_preview_session(&self, session_id: String) -> Option<PreviewSession> {
        let sessions = self.preview_sessions.read().await;
        sessions.get(&session_id).cloned()
    }
    
    /// List preview sessions
    pub async fn list_preview_sessions(&self) -> Vec<PreviewSession> {
        let sessions = self.preview_sessions.read().await;
        sessions.values().cloned().collect()
    }
    
    /// Stop preview session
    pub async fn stop_preview_session(&self, session_id: String) -> Result<(), LivePreviewError> {
        {
            let mut sessions = self.preview_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = PreviewStatus::Stopped;
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.active_sessions -= 1;
        }
        
        Ok(())
    }
    
    /// Get performance metrics
    pub async fn get_metrics(&self) -> LivePreviewMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
}

/// Supporting implementations
impl PreviewPerformanceStats {
    pub fn default() -> Self {
        Self {
            page_load_time: std::time::Duration::from_millis(1200),
            first_contentful_paint: std::time::Duration::from_millis(800),
            largest_contentful_paint: std::time::Duration::from_millis(1000),
            cumulative_layout_shift: 0.1,
            first_input_delay: std::time::Duration::from_millis(50),
            memory_usage: MemoryUsage {
                used_heap_size: 10 * 1024 * 1024,
                total_heap_size: 20 * 1024 * 1024,
                heap_size_limit: 100 * 1024 * 1024,
                external_memory: 5 * 1024 * 1024,
            },
            network_requests: Vec::new(),
            javascript_errors: Vec::new(),
            console_logs: Vec::new(),
        }
    }
}

/// Live Preview error types
#[derive(Debug, thiserror::Error)]
pub enum LivePreviewError {
    #[error("Session not found")]
    SessionNotFound,
    #[error("Server startup failed")]
    ServerStartupFailed,
    #[error("File watching failed")]
    FileWatchingFailed,
    #[error("Element inspection failed")]
    ElementInspectionFailed,
    #[error("Live edit failed")]
    LiveEditFailed,
    #[error("Hot reload failed")]
    HotReloadFailed,
    #[error("Sync failed")]
    SyncFailed,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_live_preview_creation() {
        let preview_system = LivePreviewSystem::new();
        let metrics = preview_system.get_metrics().await;
        assert_eq!(metrics.total_sessions, 0);
    }
    
    #[tokio::test]
    async fn test_preview_session_creation() {
        let preview_system = LivePreviewSystem::new();
        
        let session_id = preview_system.start_preview_session(
            "Test Preview".to_string(),
            PathBuf::from("/project"),
            PathBuf::from("/project/src/index.html"),
            PreviewFramework::React,
            PreviewConfiguration {
                port: 3000,
                host: "localhost".to_string(),
                auto_reload: true,
                hot_module_replacement: true,
                source_maps: true,
                css_injection: true,
                js_injection: true,
                error_overlay: true,
                performance_monitoring: true,
                accessibility_checks: true,
                responsive_testing: true,
                browser_sync: true,
                custom_headers: HashMap::new(),
                proxy_rules: Vec::new(),
            },
        ).await.unwrap();
        
        assert!(!session_id.is_empty());
        
        let session = preview_system.get_preview_session(session_id).await;
        assert!(session.is_some());
        
        let session = session.unwrap();
        assert_eq!(session.session_name, "Test Preview");
        assert!(matches!(session.framework, PreviewFramework::React));
        assert!(matches!(session.status, PreviewStatus::Running));
    }
}
